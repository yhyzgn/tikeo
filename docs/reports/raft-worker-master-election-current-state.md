# Raft 与 Worker Master 选举现状复盘清单

**日期:** 2026-05-30  
**范围:** tikee server 集群 Raft、worker 服务集群 master 选举、调度有序性与无额外锁机制约束。  
**目标:** 用表格清单记录现状、生产级缺口、完成状态与验证证据，便于后续功能测试复盘。

## 1. 总体状态

| 编号 | 模块 | 复盘项 | 当前状态 | 完成标识 | 代码/文档证据 | 功能测试关注点 |
|---|---|---|---|---|---|---|
| S1 | tikee server 集群 | raft-rs runtime 基础设施：`RawNode`、ticker、HTTP transport、HardState/log/snapshot/ConfState 持久化 | 已具备 | ✅ 已完成 | `crates/tikee-server/src/cluster/raft_rs.rs`、`crates/tikee-storage/src/repository/raft.rs` | 3 节点启动后能交换 raft 消息；重启后 HardState/log 恢复 |
| S2 | tikee server 集群 | 自主 master election：runtime 无已知 leader 时自主触发 campaign，不依赖人工 API 或测试 harness | 本轮已补齐 | ✅ 已完成 | `crates/tikee-server/src/cluster/raft_rs.rs` 的 autonomous campaign；`raft_inprocess_harness_autonomously_elects_unique_leader_after_ticks` | 多节点同时启动时只能出现一个 leader；leader 故障后可重新选主 |
| S3 | tikee server 集群 | 调度 gate：只有 raft leader 且 leader fencing token 已持久化时 `can_schedule=true` | 已具备并保留 | ✅ 已完成 | `update_runtime_status`；`ClusterStatus.can_schedule`；`leader_fencing_token` | follower/candidate 不应触发 tick/dispatch/alert retry |
| S4 | tikee server 集群 | Membership proposal gate：成员变更必须由真实 leader + RBAC + fencing 进入 raft ConfChange | 已具备 | ✅ 已完成 | `/api/v1/raft/members:propose`、`raft_inprocess_membership_proposal_commits_and_applies_member` | 非 leader 提案返回拒绝；移除 leader / 破坏 quorum 被阻断 |
| W1 | worker 服务集群 | Worker session lifecycle：logical instance、generation、fencing token、heartbeat lease、replaced/stopped/offline | 已具备 | ✅ 已完成 | `crates/tikee-server/src/tunnel/registry.rs`、`crates/tikee-storage/src/repository/worker_lifecycle.rs` | 同一实例重启后旧 generation heartbeat 被拒绝 |
| W2 | worker 服务集群 | 结构化 master election 声明：注册协议中明确 `WorkerClusterElection`，禁止靠字符串约定表达 master | 本轮已补齐 | ✅ 已完成 | `crates/tikee-proto/proto/worker.proto`、`sdks/java/.../worker.proto`、`sdks/rust/tikee/proto/worker.proto` | 注册包中应含 `election.enabled/domain/priority` |
| W3 | worker 服务集群 | Worker election domain：默认按 `namespace/app/cluster/region` 形成选举域，可显式覆盖 domain | 本轮已补齐 | ✅ 已完成 | `WorkerRegistry` 的 `normalized_election_domain` / `worker_domain` | 不同 namespace/app/cluster/region 不应互相抢 master |
| W4 | worker 服务集群 | Domain 内唯一 master：在线且 lease 有效 worker 中按 priority + worker_id 确定唯一 master | 本轮已补齐 | ✅ 已完成 | `recompute_worker_master_states`；`registry_elects_single_master_per_worker_domain_and_fails_over` | 多 worker 同域时仅一个 `isMaster=true` |
| W5 | worker 服务集群 | Worker master failover：下线、transport error、unregister、generation replacement 后重新选 master | 本轮已补齐 | ✅ 已完成 | `mark_transport_error`/`unregister` 后重算 master；worker 测试覆盖 | 当前 master 断开后 follower 自动晋升 |
| W6 | worker 服务集群 | Worker API 可观测性：列表返回 master domain、isMaster、masterWorkerId、term、fencingToken | 本轮已补齐 | ✅ 已完成 | `WorkerMasterSummary`、`routes/workers.rs`、`web/src/api/client.ts` | `/api/v1/workers` 返回字段完整，UI 可显示 Master/Follower |
| D1 | 调度有序性 | Server 侧调度只能由 server Raft leader 执行 | 已具备并保留 | ✅ 已完成 | `tikee.rs`、`tunnel/dispatcher.rs`、`alert/retry.rs` 中 `can_schedule` gate | follower 节点不能 claim dispatch queue |
| D2 | 调度有序性 | 普通 single dispatch 优先派发给对应 worker domain master，降低同服务集群多节点乱序风险 | 本轮已补齐 | ✅ 已完成 | `find_ordered_dispatch_workers`；`dispatcher.rs` 改用 master-first candidates | 同域 master 在线时新任务优先落 master |
| D3 | 调度有序性 | Broadcast fan-out 保持向所有匹配 worker 发送，同时继续使用 assignment token/fencing | 已保留 | ✅ 已完成 | `find_eligible_workers_with_broadcast_selector`、assignment token 校验 | 广播任务不应被 master-only 限制 |
| SDK1 | Java SDK | 注册请求携带结构化 worker election 信息，默认启用 | 本轮已补齐 | ✅ 已完成 | `WorkerClusterElection.java`、`WorkerRegistration.java`、`GrpcTikeeWorkerClient.java` | Java demo 启动后服务端可看到 election 信息 |
| SDK2 | Spring Boot Starter | 自动配置暴露 `tikee.worker.election.enabled/domain/priority`，默认启用 | 本轮已补齐 | ✅ 已完成 | `TikeeWorkerProperties.java`、`TikeeWorkerAutoConfiguration.java` | application.yml 可覆盖 election domain/priority |
| SDK3 | Rust SDK | 保持可编译，注册信息同步带默认 election 声明 | 本轮已补齐 | ✅ 已完成 | `sdks/rust/tikee/src/config.rs` | `cargo check` 通过；后续再补完整 ergonomic API |
| UI1 | Web Worker 页面 | Worker 列表展示 Master/Follower 与 election domain | 本轮已补齐 | ✅ 已完成 | `web/src/pages/workers/WorkerTable.tsx` | 多 worker 同域时 UI 能区分 master/follower |
| DOC1 | 文档 | 总设计文档同步 server/worker 双 master election 方案 | 本轮已补齐 | ✅ 已完成 | `design/tikee-architecture-design.md` | 后续路线图不应再把 worker master 选举视为未设计 |
| DOC2 | 覆盖报告 | 覆盖报告同步 WorkerClusterElection 与 master-first dispatch 状态 | 本轮已补齐 | ✅ 已完成 | `docs/reports/feature-coverage-competitive-checklist.md` | 功能覆盖复盘能看到该能力已闭环 |

## 2. 生产级验收清单

| 编号 | 验收项 | 完成标识 | 已运行验证 | 复盘建议 |
|---|---|---|---|---|
| V1 | Server Raft 测试通过，包含无手动 campaign 的自主选主测试 | ✅ 已完成 | `rtk cargo test -p tikee-server raft -- --nocapture`：30 passed | 后续补 Docker/K8s 多进程 chaos |
| V2 | Worker registry 测试通过，包含 master 初选、故障切换、master-first dispatch candidates | ✅ 已完成 | `rtk cargo test -p tikee-server worker -- --nocapture`：14 passed | 后续增加 lease 过期定时扫描 e2e |
| V3 | Rust SDK 保持可编译 | ✅ 已完成 | `rtk cargo check -p tikee --manifest-path sdks/rust/tikee/Cargo.toml` | 后续补 Rust SDK election 配置 API |
| V4 | Java SDK 注册请求包含结构化 election 信息 | ✅ 已完成 | `rtk bash -lc 'cd sdks/java && ./gradlew :tikee:test --tests com.yhyzgn.tikee.worker.client.GrpcTikeeWorkerClientTest'` | 后续跑 live Spring demo 连接本地 server |
| V5 | Web 侧类型与 Worker 页面回归通过 | ✅ 已完成 | `rtk npm --prefix web test -- --run`：54 passed | 后续用真实双 worker 截图验收 UI 状态 |
| V6 | 文档 diff 无空白错误 | ✅ 已完成 | `rtk git diff --check` | 后续每次改 election 行为都更新本清单 |

## 3. 尚需后续环境级复盘的项目

| 编号 | 项目 | 当前状态 | 完成标识 | 原因 | 建议测试方式 |
|---|---|---|---|---|---|
| E1 | 多进程 Docker/K8s Raft chaos | 未在本轮本地执行 | ⏳ 待环境级验证 | 当前验证为 in-process harness + 单元/集成测试，未实际拉起 3 个 server 进程 | 使用 `scripts/raft-bridge-e2e.sh` 或 K8s StatefulSet 做 leader failover/网络分区测试 |
| E2 | Live Java Spring demo worker failover | 未在本轮本地执行 | ⏳ 待环境级验证 | Java SDK 单测覆盖注册包，未同时启动多个 demo 实例连真实 server | 启动 2-3 个 Java demo，同 namespace/app/cluster/region，观察 Worker 页面 master 切换 |
| E3 | Lease 过期驱动的 worker master 自动重选 | 基础函数具备，缺端到端时间推进验证 | ⏳ 待增强验证 | 当前测试覆盖 transport error/unregister/replacement，未用真实时间等待 lease 过期 | 增加可注入 clock 或短 lease e2e 测试 |
| E4 | 生产级 raft snapshot/compaction/large log 压测 | 仍是长期硬化项 | ⏳ 待增强验证 | 本轮聚焦 election 和 ownership gate，不做大规模日志压缩压测 | 构造大量 membership/noop command，验证 snapshot/restore/compaction |

## 4. 功能测试建议顺序

| 顺序 | 场景 | 预期结果 |
|---|---|---|
| 1 | 启动单 server standalone | `role=standalone`，可调度 |
| 2 | 启动 3 server raft | 只有 1 个 leader，且只有 leader `canSchedule=true` |
| 3 | 停止 raft leader | 新 leader 产生，旧 leader 不再调度 |
| 4 | 启动 2 个同 domain Java worker | Worker 页面显示 1 个 Master、1 个 Follower |
| 5 | 手动触发 single job | 优先派发到 Master worker |
| 6 | 停止 Master worker | Follower 晋升 Master，后续 single job 派发到新 Master |
| 7 | 触发 broadcast job | 所有匹配 worker 都收到任务，不受 master-only 限制 |
| 8 | 重启同 clientInstanceId worker | 新 generation 生效，旧 generation heartbeat/result 被 fencing |
