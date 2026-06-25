# 进度记录

## 当前状态

- [x] 输出独立文档站搭建方案：`design/docs-site-build-plan.md`（方案阶段，尚未搭建/部署）
- [x] 架构设计文档完成：`design/tikeo-architecture-design.md`
- [x] 移除旧版本/v2 表述，保留功能内容
- [x] 补充多语言动态脚本与安全沙箱设计
- [x] 补充 K8s/Docker/跨集群部署与 Worker Tunnel 网络穿透设计
- [x] 补充 Web UI 与 HTTP/OpenAPI 管理接口设计
- [x] 创建开发阶段总提示词：`prompt.md`
- [x] 初始化 `.memory` 记忆库
- [x] 初始化 `.prompt` 阶段提示词目录
- [x] 固化 Rust workspace + `./crates/` 解耦约束
- [x] 固化 Web 端 `./web` + React + Ant Design + Bun 约束
- [x] 固化依赖尽量使用当前最新稳定版的约束

## 下一大阶段

进入代码开发：`001-bootstrap` 至 `013-broadcast-execution` 已完成；下一阶段执行 `014-worker-capability-routing`。

- [x] 001-bootstrap：初始化 Cargo workspace 与 `./crates/*` crate 骨架
- [x] 001-bootstrap：实现 `tikeo serve`、`/healthz`、`/readyz`
- [x] 001-bootstrap：通过 fmt、clippy、test、build 与 healthz/readyz 冒烟
- [x] 002-http-api-and-openapi：HTTP 管理 API 与 OpenAPI 3.1
- [x] 002-http-api-and-openapi：选择 `utoipa`；禁止 API 文档 UI 依赖
- [x] 002-http-api-and-openapi：实现 `/api/v1/system/info`、`/api/v1/cluster`、Jobs skeleton
- [x] 002-http-api-and-openapi：暴露 `/api-docs/openapi.json`；不提供文档 UI
- [x] 002-http-api-and-openapi：后端入口调整为根 `src/main.rs`，业务模块继续在 `crates/*`
- [x] 003-worker-tunnel：Worker 主动连接与注册心跳
- [x] 固化 HTTP 业务接口统一 `{code,message,data}` 响应规范
- [x] 已在设计文档开发路线图标记完成项：脚手架、HTTP API skeleton、OpenAPI JSON、CLI serve
- [x] 路线图完成项标记规范调整为仅使用 `[x]`，不额外使用 ✅ 图标
- [x] Java SDK 规划补充：优先支持 Spring Boot Starter 模式
- [x] 003-worker-tunnel：新增 `tikeo-proto` crate 与 Worker Tunnel protobuf
- [x] 003-worker-tunnel：实现 server 侧 Worker Tunnel gRPC skeleton 与内存 registry
- [x] 003-worker-tunnel：server 同时启动 HTTP 9090 与 Worker Tunnel gRPC 9998
- [x] 设计路线图标记：gRPC 协议定义与代码生成
- [x] 004-storage-and-tikeo：SeaORM 存储层、SQLite dev DB、MySQL migration feature、Jobs API 持久化
- [x] 005-basic-tikeo：调度领域模型、API 手动触发实例链路、实例列表查询
- [x] 006-worker-sdk-rust-and-java-starter：Rust Worker SDK 注册/心跳客户端 + Java Spring Boot Starter 骨架
- [x] 007-web-ui-foundation：Web 管理端基础工程、Job/Instance 页面骨架
- [x] 008-container-deployment：Docker / Compose / K8s 部署基础
- [x] 009-worker-dispatch：Worker Tunnel 真实任务分发、执行回传与实例状态流转
- [x] 010-tikeo-tick-loop：CRON / Fixed Rate tick loop 与调度触发
- [x] 011-instance-logs：实例执行日志与 Web 日志查看基础
- [x] 012-auth-rbac-foundation：登录与权限感知操作基础
- [x] 013-broadcast-execution：广播执行基础
- [x] 014-worker-capability-routing：Worker 能力 / 标签 / namespace / app 基础路由
- [x] 015-user-management-and-rbac：账号体系、用户管理、RBAC 权限验证与 SessionStore 抽象
- [x] 016-dynamic-script-sandbox：脚本定义 CRUD（storage + migration + repository + HTTP API + OpenAPI）、ScriptLanguage/ScriptStatus 核心类型、Web 脚本管理页面
  - [x] 脚本版本历史（`script_versions` 表）、更新自动产生版本记录
  - [x] 版本 diff 对比 API 与 Web diff 视图
  - [x] 脚本编辑器语法高亮（CodeMirror 6，Shell/Python/Node）
- 023 Phase2 workflow visual/mapreduce：executor 推进器、Map/MapReduce/子工作流定义约束、dry-run/advance API、Web DAG/SSE 基础已开发，等待完整验证。

## 2026-05-20 — 024 Phase2 distributed worker/recovery slice

- Workflow queued node 与执行链路打通：`materialize_next_queued_node` 可把 job 节点生成 job_instance + dispatch_queue，把 map/map_reduce 节点生成 workflow_shards，把 sub_workflow 节点生成 child workflow_instance 软关联。
- 新增 workflow_shards 表；workflow_node_instances 增加 child_workflow_instance_id，继续无外键，仅软关联。
- 新增恢复 API：`POST /api/v1/workflow-instances/{id}/recover`，支持 retry/skip/fail/succeed 最小语义。
- 新增 Worker/队列管理 API：`GET /api/v1/workers`、`GET /api/v1/dispatch-queue`，Web 新增 Worker 集群页面。
- Dispatcher loop 每轮尝试 materialize 一个 queued workflow node，再走既有 job/broadcast dispatch。

- Workflow UI upgraded from preview-only to a lightweight visual editor: draggable node ordering, node/edge add-delete-edit, JSON sync, and existing dry-run/create path preserved.

## 2026-05-20 — Blender-like workflow node canvas

- Workflow visual editor upgraded from list/card layout to node-canvas style: grid canvas, absolute-positioned nodes, left input/right output ports, SVG Bézier arrow edges, port-click connection flow, and free node dragging with coordinates stored under node.config.ui.
- JSON remains canonical; visual coordinates and edges are synced back into WorkflowDefinition.

## 2026-05-20 — Workflow canvas layout/connection fixes

- Removed the wide left definition panel from Workflows page; creation now only needs inline name + create button, and JSON/YAML/Dry-run are canvas card actions.
- Fixed YAML preview by deriving YAML from current parsed JSON without replacing canonical draft state.
- Fixed node port connection by stopping pointer propagation on ports, and added per-node-type in/out connection limits shown on each node.

## 2026-05-20 — Workflow port linking reliability fix

- Fixed workflow canvas linking reliability by triggering port actions on pointerdown as well as click, enlarging the invisible port hit area, cancelling drag state during linking, and highlighting the source node while linking.
- Temporarily relaxed per-type port limits to 8 in/out for all node types to avoid UX-blocking false negatives while the exact product semantics are still evolving.

## 2026-05-20 026 补充：工作流节点画布拖拽连线
- Workflows 可视化编辑器支持节点边缘端口 hover 显示；拖拽输出端口时所有端口可见，画布显示临时虚线箭头，释放到目标输入端口完成连线。
- 端口连线按节点类型限制入/出度；start 仅输出，end 仅输入，condition/parallel/join 等有独立限制。
- 节点目录补齐 start/end/job/script/http/condition/parallel/join/delay/approval/notification/map/map_reduce/sub_workflow。
- 后端 workflow definition 校验同步放行上述节点类型；job/sub_workflow/map/map_reduce 的必填约束保持不变。

## 2026-05-20 027：工作流节点属性配置
- Workflows 画布新增节点属性面板：点击节点后可编辑 key/name，并按节点类型配置业务语义。
- Job 节点可从已创建任务列表中绑定 job_id；Script/HTTP/Condition/Parallel/Join/Delay/Approval/Notification/Map/SubFlow 均有对应配置入口。
- 后端 workflow 校验新增部分节点必填配置检查：condition.expression、http.url、script.source、approval.approvers；job/sub_workflow/map/map_reduce 原约束保持。

## 2026-05-20 028：工作流连线选择与重连
- Workflows 画布已移除“连接最后两个节点”快捷按钮，连线统一通过端口拖拽创建。
- SVG 边增加透明 hit path：点击箭头线条可选中，选中后显示起点/终点 handle。
- 按住连线两端 handle 可拖动重连到其他节点端口，用于调整边的 from/to 节点对象。

## 2026-05-20 029：边关系线条内联配置
- Workflows 画布移除底部“边关系”列表式编辑区域。
- 点击连线后，在线条中点附近显示浮层，可直接修改 edge condition（on_success/on_failure/always）并删除连线。
- 连线端点拖拽重连能力保留，浮层提示可拖动两端圆点调整连接对象。

## 2026-05-20 030：边条件按前置节点语义生成
- Workflows 画布的边条件候选项改为根据前置节点类型生成：condition 显示 true/false，approval 显示 approved/rejected/always，parallel 默认 branch/always，HTTP/Script 显示成功/失败/完成等语义。
- 新建连线默认条件取前置节点的首选输出语义；重连起点时自动切换为新前置节点默认条件。
- 连线颜色按条件语义变化，并在线条上弱显示当前 condition 文本；点击画布空白处会关闭边关系浮层。

## 2026-05-20 031：修复边端点重连层级
- 修复点击线条后只能编辑条件、无法拖动两端的交互回归。
- 选中边时除 SVG ghost handle 外，额外渲染高 z-index HTML 端点按钮，确保不被节点卡片、SVG 层级或条件浮层遮挡。
- 端点按钮继续复用原重连逻辑：拖起点改 from，拖终点改 to。

## 2026-05-20 032：监听地址与普通边默认关系调整
- 普通节点新建边默认关系改为 `always`；特定节点仍使用各自语义默认值（如 condition true/on_success、approval approved/on_success、parallel always）。
- 项目默认监听地址统一从 `127.0.0.1` 改为 `0.0.0.0`，覆盖配置、脚本、Vite proxy、README、prompt 和记忆库命令。
- Worker Tunnel 端口统一从 `9091` 改为 `9998`，覆盖 Rust 默认配置、dev/container/k8s/docker-compose/Dockerfile、Rust/Java SDK 默认 endpoint 与文档。

## 2026-05-20 033：工作流页面列表优先与编辑页拆分
- `/workflows` 一级页面调整为工作流列表优先，不再默认展示画布编辑器、运行视图和事件流。
- 新增 `/workflows/new` 与 `/workflows/:id/edit` 路由，使用独立编辑页承载节点画布、JSON/YAML、Dry-run、创建/保存。
- 列表每个 item 操作栏新增“运行视图”按钮；运行视图与实例事件流通过 Collapse 手风琴展开，只展示一个条目的运行详情。
- 后端新增 `PATCH /api/v1/workflows/{id}` 支持编辑保存工作流定义，并同步重建 workflow_node/workflow_edge 软关联记录。

## 2026-05-20 034：工作流运行按钮文案与空队列提示
- 将“物化下一节点”文案改为“准备下一节点执行”，降低工程术语暴露。
- 当后端返回 no queued workflow node found（无可准备节点）时，前端不再直接展示 404，而是提示“当前没有等待准备的节点：请先运行工作流，或先推进已有运行中节点”。
- “推进首个队列节点”按钮文案改为“标记当前节点成功”。

## 2026-05-20 035：工作流运行视图条目内联展开
- Workflows 列表移除全局 Collapse 生成的“运行视图 · name”标题和占位提示，避免条目多时列表混乱。
- 点击某条工作流的“运行视图”按钮后，运行视图与实例事件流直接内联展示在该条目下方；再次点击收起，并保持一次只展开一个条目的手风琴行为。
- 切换到其他工作流的运行视图时清空旧实例、事件与 shard 展示，避免把上一个工作流运行态误挂到新条目下。

## 2026-05-20 036：工作流二级页返回入口
- Workflow 新增/编辑二级页面在顶部 hero 区域增加“返回工作流列表”按钮，进入二级页面后可直接回到 `/workflows`。
- 移除画布卡片操作区里的重复返回按钮，让顶部导航承担页面级返回职责，工具区保留预览模式与 Dry-run。

## 2026-05-20 037：运行视图只读化
- Workflows 列表内联展开的运行视图明确进入只读模式：不渲染节点端口、不允许拖拽节点、不允许点击/编辑/删除/重连线条。
- WorkflowEditorPage 仍通过 `editable` 模式保留完整节点、端口、边条件与重连编辑能力。

## 2026-05-20 038：Worker 结果自动推进工作流
- 025 阶段启动：Worker Tunnel 收到 `TaskResult` 后，除更新 job_instance / broadcast attempt 外，会按 job_instance_id 软关联查找 workflow_node_instance。
- job 节点结果自动映射为 workflow node `succeeded` / `failed`，并调用 workflow advance 按边条件入队后继节点，减少列表运行视图中的手动推进依赖。
- dispatch_queue 新增 `lease_owner` / `lease_until` 字段与 SQLite 兼容迁移，API queue summary 也返回这两个字段，为后续原子 claim / visibility-timeout 打基础。

## 2026-05-20 039：工作流操作审计补齐
- 工作流 HTTP 管理动作补齐 audit log：create/update/validate/dry-run/run/advance/materialize/recover。
- 工作流集成测试增加审计断言，确认 workflow / workflow_instance / workflow_node_instance 相关动作写入审计日志。

## 2026-05-20 040：SDK 目录统一
- Rust Worker SDK 从 `crates/tikeo` 迁移到 `sdks/rust/tikeo`，Cargo workspace 显式包含该路径。
- Java Spring Boot Starter SDK 从 `java/` 迁移到 `sdks/java/`；后续已改为 Gradle 验证命令 `./sdks/java/gradlew -p sdks/java test`。
- Dockerfile、README、gitignore、design、prompt 和 memory 中的 SDK 路径引用已同步更新。

## 2026-05-21 041：Dispatch Queue 租约 Claim API
- dispatch_queue 在已有 lease_owner / lease_until 字段基础上新增 repository claim/release 能力：claim 会设置租约 owner、过期时间并递增 attempt。
- 新增 `POST /api/v1/dispatch-queue:claim`，需要 workers manage 权限；成功 claim 会写入 audit log，便于追踪多 server/worker 对队列项的占用。
- 增加存储层测试覆盖 claim、重复 claim 阻止、release 后重新 claim 与 attempt 递增。

## 2026-05-21 042：开发脚本本地访问 URL 覆盖
- 用户手动调整 `scripts/dev.sh`：新增 `TIKEO_API_PORT` / `TIKEO_WEB_URL` 可配置项。
- dev 脚本默认仍让后端按配置绑定 `0.0.0.0`，但健康检查与浏览器提示默认使用 `http://localhost:<port>`，更符合本机开发访问习惯。
- 验证：`bash -n scripts/dev.sh`；`timeout 10 ./scripts/dev.sh` 可成功启动后端与 Web，并在超时信号下清理进程。

## 2026-05-21 043：Dispatch Queue 原子 Claim 与 Dispatcher 接入
- 单实例 job 创建时同步写入 dispatch_queue 软关联队列行，dispatcher 不再直接扫描 pending job_instance，而是通过 dispatch_queue claim/lease 抢占后再派发。
- `claim_dispatch_queue_item` 改为 DB 条件更新：只有 pending 且无未过期 lease 的行能被抢占，同时原子递增 attempt，避免多 server 并发重复 claim。
- Workflow queued node materialize 也改为先 claim lease，再物化节点；物化完成后将原 workflow-node queue 行标记 done 并清理 lease。
- dispatcher 每轮会清理过期 pending lease；worker 不可用或实例状态已变化时会 release lease 并恢复 pending，成功派发后把 job queue 行标记 running。

## 2026-05-21 044：Workflow Shard 聚合与子工作流回写
- workflow_shards 新增 `job_instance_id` 软关联字段；map / map_reduce materialize 时会为每个 shard 创建 job_instance 与 dispatch_queue 行，使 shard 可进入 worker 派发链路。
- 新增 `CompleteWorkflowShardInput/Result` 与 `POST /api/v1/workflow-shards/{id}/complete`：写入 shard status/output/event，全部 shard 成功后自动 advance 当前 map 节点，任一 shard failed 时按失败边推进。
- Worker TaskResult 回写扩展到 shard job_instance：若 job_instance 关联 shard，则先完成 shard，再由 shard 聚合决定是否推进 workflow node。
- sub_workflow materialize 会初始化子工作流节点与起始 dispatch_queue；子工作流完成后自动回写父 sub_workflow 节点终态并推进父后继。

## 2026-05-21 045：SDK/examples 目录规范重规划
- 规划 `sdks/<language>/<sdk-name>` 结构，Rust SDK 从旧 `sdks/tikeo` 迁移到 `sdks/rust/tikeo`。
- Java SDK 规划改为 Gradle 多模块 + JDK 21+，替换 Maven 骨架并统一使用 `./sdks/java/gradlew -p sdks/java test` 验证命令。
- 新增 `examples/<language>/<demo-name>` demo 目录规范；后续开发过程中由 AI 自主判断何时创建 demo 来调试 SDK/Worker/工作流集成链路。

## 2026-05-21 046：SDK 目录整改执行
- Rust Worker SDK 已迁移为 `sdks/rust/tikeo`，Cargo workspace 已同步；服务端 Dockerfile 已移除 SDK 处理。
- Java SDK 已移除 Maven `pom.xml` 骨架，新增 Gradle Kotlin DSL 多模块构建，统一 JDK 21 toolchain / release。
- 新增 `examples/<language>/<demo-name>` demo 目录骨架与 README；后续 SDK/Worker/工作流调试可按需扩展 runnable demo。
### 2026-05-21 SDK layout correction follow-up
- 用户明确根 `Dockerfile` 只构建 tikeo 服务端；已约束不得复制/缓存/构建 `sdks/` 或 `examples/`。
- SDK 路径规范固定为 `sdks/<language>/<sdk-name>/`，Demo 路径规范固定为 `examples/<language>/<demo-name>/`。
- Rust SDK 路径为 `sdks/rust/tikeo`；现已移除 repo-local path dependencies，满足独立发布约束。
- 已补齐可独立运行的 Rust demo（`examples/rust/worker-demo`）与 Java Spring Boot demo（`examples/java/spring-worker-demo`）基础。
- Dockerfile 继续保持服务端专用，构建阶段改为 Alpine Rust 镜像并使用 Alpine runtime，避免 SDK/Demo 进入镜像上下文。

### 2026-05-21 Rust SDK independent publishing cleanup
- Removed `sdks/rust/tikeo` from root Cargo workspace and removed Dockerfile rewrite workaround.
- Made Rust SDK self-contained by bundling `proto/worker.proto`, local `build.rs`, and removing all `../../../crates/*` path dependencies.
- Replaced SDK integration tests with an in-crate mock Worker Tunnel server.

### 2026-05-21 Worker identity assignment cleanup
- Changed Worker Tunnel RegisterWorker payload from client-supplied `worker_id` to optional `client_instance_id`.
- Server registry now generates authoritative `wrk-*` worker ids and returns them in `WorkerRegistered`.
- Rust SDK stores server-assigned worker id after connect and uses it for heartbeat/log/result messages.
- Worker identity cleanup verification completed for Rust workspace, standalone Rust SDK, Java SDK, and Java demo. Java wrapper download hit network EOF once, then verification passed with cached Gradle 8.14 binary.

## 2026-05-21 047：Java SDK Worker Tunnel
- Java Core SDK 新增 protobuf/gRPC 生成，内置 `GrpcTikeoWorkerClient`，支持 OpenTunnel 注册、读取服务端下发 worker_id、定时心跳、任务日志和任务结果回传。
- Spring Boot Starter auto-configuration 默认创建真实 gRPC client；新增 `tikeo.worker.dry-run` 让 demo/测试无需 live tikeo。
- Java Spring demo 默认 dry-run，可通过配置切换到 live Worker Tunnel。

## 2026-05-21 048：Java TikeoProcessor 适配
- Spring `TikeoProcessorRegistry` 已从 bean map 升级为 invocable handler registry，拒绝重复 processor name。
- 新增 `SpringTikeoTaskProcessor`，当前按 `TaskContext.jobId()` 匹配 `@TikeoProcessor` 名称，支持 `TaskContext` / `String` / `byte[]` 入参和 `TaskOutcome` / `String` / `boolean` / `void` 返回。
- Spring Boot auto-configuration 已把真实 gRPC client 接到 registry adapter，demo 的 `demo.echo` 可作为真实 processor 方法被调用。

## 2026-05-21 049：Java SDK 三模块重组
- Java SDK 已按用户要求重组为 3 个 Gradle 子模块：`tikeo`、`tikeo-spring`、`tikeo-spring-boot-starter`。
- Spring Framework 的 `@TikeoProcessor` registry/adapter 独立在 `tikeo-spring`；Spring Boot Properties/AutoConfiguration/starter 聚合在 `tikeo-spring-boot-starter`。
- Java demo 依赖已切换到 `net.tikeo:tikeo-spring-boot-starter`。

- Spring Boot Java SDK module renamed to `tikeo-spring-boot-starter` per user naming correction; demo dependency updated accordingly.

## 2026-05-21 050：Worker processor key protocol
- Worker Tunnel `DispatchTask` proto 新增 `processor_name` 字段，并同步到服务端 proto、Rust SDK proto、Java SDK proto。
- Server dispatcher 分发任务时填充 `processor_name`，当前兼容性默认等于 `job_id`。
- Rust/Java TaskContext 暴露 processor name；Java Spring adapter 改为优先按 `processorName()` 路由 `@TikeoProcessor`。

## 2026-05-21 051：Job / Workflow processor 绑定模型
- Job 定义新增可选 `processor_name`，HTTP create/list/OpenAPI DTO 与 Web Job 表单/列表同步展示。
- Workflow `WorkflowNodeSpec` 新增可选 `processor_name`，job/map 节点 UI inspector 可配置 SDK processor 绑定。
- Dispatcher processor 解析顺序固定为：Workflow 节点 processor -> Job processor -> legacy job_id，避免 SDK 路由继续依赖任务 ID。
- SQLite 兼容迁移补齐 `jobs.processor_name` 与 `workflow_nodes.processor_name`，无外键规则保持不变。

### 2026-05-21 Go/Python SDK deferral
- User explicitly moved Go SDK + Python SDK out of Phase 2 and into Phase 4.
- Current Phase 2 continuation target is realtime task log streaming over gRPC server stream.

### 2026-05-21 Phase2 realtime task log stream
- Worker Tunnel proto now exposes `SubscribeTaskLogs(SubscribeTaskLogsRequest) returns (stream TaskLog)`.
- Server replays persisted `job_instance_logs` after a requested sequence and then streams live TaskLog records via an in-memory broadcast fan-out after successful DB append.
- Go/Python SDK remains deferred to Phase4 per user instruction.

### 2026-05-21 Phase2 PostgreSQL/CockroachDB storage support
- Enabled `sqlx-postgres` on `tikeo-storage` and migrations so PostgreSQL URLs compile through SeaORM/sqlx.
- Added `config/postgres.toml` with PostgreSQL and CockroachDB URL examples; CockroachDB uses PostgreSQL wire protocol.
- Roadmap marks PostgreSQL + CockroachDB storage support complete at driver/config/template level; live DB smoke remains environment-dependent.

### 2026-05-21 Phase2 cluster coordinator foundation
- Added `tikeo-server::cluster` with ClusterCoordinator trait, explicit ClusterMode/ClusterRole, and StandaloneCoordinator.
- `/api/v1/cluster` now reports `role=standalone` with node_id/can_schedule/detail instead of fake `leader`.
- Design now records Raft implementation boundaries: leader ownership gate, follower fencing, DB claim as final idempotency guard, and container-friendly networking.

### 2026-05-21 Phase2 cluster ownership gates
- Tikeo tick loop and Worker dispatcher loop now consult `ClusterCoordinator` status before ownership-sensitive work.
- Standalone remains schedulable; mock Raft follower tests prove tick and dispatch skip work when `can_schedule=false`.
- dispatch_queue DB conditional claim remains in place as final idempotency/fencing guard.

### 2026-05-21 Phase2 Raft config shape
- Added `[cluster]` config with `mode`, `node_id`, and static `peers` shape.
- Server now builds ClusterCoordinator from config: standalone can schedule; raft mode reports unknown/not-schedulable until real consensus starts.
- Added `config/raft.toml` as a safe template; no fake leader behavior introduced.

### 2026-05-21 Phase2 Raft metadata persistence
- Checked crates.io on 2026-05-21: OpenRaft alpha/prerelease conclusion was superseded by user direction to use TiKV raft-rs; real runtime adoption remains gated on event-loop/transport/persistence/fencing work.
- Added `raft_metadata` and `raft_members` storage tables with no foreign keys; IDs remain soft-linked.
- Raft startup now persists local metadata and configured peers, but cluster status remains unknown/not-schedulable until real consensus produces leadership.

### 2026-05-21 Phase2 Raft transport/fencing shape
- Added leader fencing token field shape to cluster status and `raft_metadata`; placeholder/config paths keep it null.
- Added reserved `/api/v1/raft/append-entries` HTTP transport endpoint for Docker/K8s/LB-safe node-to-node wiring; it returns `accepted=false` until real consensus runtime exists.
- Kept current storage-backed no-op coordinator in `tikeo-server::cluster`; no new `tikeo-cluster` crate yet because runtime boundaries are not stable enough.

### 2026-05-21 Phase2 cluster diagnostics
- Added `/api/v1/cluster/diagnostics` for operator-visible cluster readiness: current status, scheduling gate, persisted Raft metadata, members, transport placeholder, and runtime boundary.
- Chose a separate diagnostics endpoint instead of bloating `/api/v1/cluster`; the lightweight status endpoint stays stable for UI polling.
- Kept cluster runtime in `tikeo-server::cluster` for now; no `tikeo-cluster` crate until consensus/runtime traits stabilize.

### 2026-05-21 Phase2 dispatch queue fencing token
- Reviewed Phase2: only full Raft runtime remains incomplete; Go/Python SDK stays Phase4.
- Added `dispatch_queue.fencing_token` shape and SQLite compatibility migration; claim responses now include a fencing token.
- Dispatcher now derives a fencing token from ClusterCoordinator status (`standalone:<node>:tikeo-dispatcher` today, future `raft:<node>:<leader-token>` when real consensus exists).

### 2026-05-21 Phase2 closeout / Phase3 audit paging
- Consensus dependency direction corrected to TiKV raft-rs (`raft` 0.7.0); full Raft scheduling still stays gated until event-loop/transport/persistence/fencing are real.
- Phase2 distributed safety foundations are documented as complete except real Raft runtime/membership.
- Started Phase3 audit governance by adding server-side audit filters and pagination plus Web UI filter controls.

### 2026-05-21 Phase2 raft-rs correction
- User corrected the OpenRaft direction; project now targets TiKV raft-rs (`raft` crate 0.7.0, Apache-2.0) instead of OpenRaft.
- Added `tikeo-server::cluster::raft_rs` bootstrap validation: deterministic string `node_id` -> non-zero u64 raft id, peer voters, `MemStorage + RawNode` construction. This proves dependency/API integration only; no tick loop, campaign, leader token, or scheduling grant exists yet.
- `mode=raft` remains `role=unknown`, `can_schedule=false`, `leader_fencing_token=null` until real raft-rs leadership/fencing is implemented.

### 2026-05-21 Phase2 raft-rs durable records and wire shape
- Added `raft_log_entries` and `raft_snapshots` tables/entities/repository helpers as no-FK durable foundations for future raft-rs Ready log/snapshot persistence.
- Updated reserved `/api/v1/raft/append-entries` request DTO to carry raft-rs message-like fields (`from/to/term/message_type/index/log_term/commit/entries/context/reject`) while still returning `accepted=false`; no consensus state mutation or leader grant yet.
- Next safe slice: implement event loop + Ready persistence/application and only derive scheduling ownership from real raft-rs leadership plus persisted fencing token.

### 2026-05-21 Phase2 raft-rs message conversion
- Added route-local conversion from the reserved Raft HTTP DTO into raft-rs `eraftpb::Message`, including message/entry type allowlists, non-negative term/index validation, and base64 decoding for message/entry payloads.
- Endpoint still returns `accepted=false` and does not enqueue/step the message; this only validates wire compatibility before the runtime loop exists.

### 2026-05-21 Phase2 raft-rs runtime ticker skeleton
- `coordinator_from_config_with_storage` now starts a `RaftRuntimeCoordinator` for `mode=raft` when bootstrap succeeds. It drives `RawNode::tick()` on a 100ms loop and processes Ready in safe order: HardState metadata, entries, snapshot, then `advance()`.
- Runtime does not campaign, does not wire outbound transport, and still keeps `can_schedule=false` and `leader_fencing_token=null`; tikeo ownership remains fenced.
- Next slice: connect validated inbound HTTP messages to the runtime inbox, then implement Ready apply/outbound transport and real leader fencing.

### 2026-05-21 Phase2 raft-rs inbound runtime inbox
- Added a `ClusterCoordinator::submit_raft_message` boundary and wired `RaftRuntimeCoordinator` to enqueue validated `eraftpb::Message` values through a bounded mpsc inbox.
- `/api/v1/raft/append-entries` now returns `accepted=true` only when a running raft-rs runtime inbox accepts the message; standalone or stopped runtimes return `accepted=false` with a clear reason. This still does not grant scheduling ownership or a leader fencing token.
- Next slice: implement outbound peer HTTP transport and Ready apply/state-machine bookkeeping before enabling any leader fencing token.

### 2026-05-21 Phase2 raft-rs outbound transport skeleton
- Added optional `cluster.transport_token` config and `x-tikeo-raft-token` support so internal Raft HTTP transport can bypass human session auth without committing production secrets.
- Wired Ready outbound messages through a `RaftPeerTransport` skeleton: raft-rs `Message` values serialize to the existing HTTP wire DTO, base64 payloads are preserved, peer URLs append `/api/v1/raft/append-entries`, and delivery runs asynchronously through reqwest.
- Tikeo ownership remains fenced: no campaign, no leader token, no `can_schedule=true`. Next slice is committed-entry apply bookkeeping and fencing-token lifecycle.

### 2026-05-21 End-of-day handoff checkpoint
- Current pushed HEAD before this checkpoint: `222b1d6 Send raft-rs outbound messages through peer HTTP skeleton 📡`; working tree was clean before writing this memory checkpoint.
- Today completed and pushed three Phase2 raft-rs slices: runtime ticker + Ready durability order (`fc67f13`), inbound HTTP -> runtime inbox (`dea7528`), and outbound peer HTTP skeleton + optional `cluster.transport_token` (`222b1d6`).
- Full verification passed after the last code slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build`.
- Next continuation prompt is `.prompt/053-phase2-raft-rs-apply-and-fencing.md`: implement Ready committed-entry apply bookkeeping (`applied_index` persistence), explicitly gate config-change entries, then design leader fencing-token lifecycle. Do not enable `can_schedule=true` from raft role alone.

### 2026-05-22 Phase2 raft-rs apply bookkeeping and fencing lifecycle
- Implemented Ready committed-entry apply bookkeeping using `advance_append` / `advance_apply_to` instead of blindly advancing without state-machine acknowledgement.
- Committed `EntryNormal` entries now monotonically update `raft_metadata.applied_index`; `EntryConfChange` / `EntryConfChangeV2` are explicitly gated and stop apply progress before silent membership mutation.
- Added leader fencing-token lifecycle: only a real raft-rs `Leader` with term > 0 derives `raft:term:<term>:node:<node_id>`, persists it first, then reports `can_schedule=true`; non-leaders clear the token. Tikeo/dispatcher gates remain driven by `can_schedule` and dispatcher uses the persisted token.
- Next slice: define business state-machine command envelope/replay idempotency and design dynamic membership handling.
- Full verification passed for this slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build`.

### 2026-05-22 Phase2 raft-rs business command envelope foundation
- Added `raft_applied_commands` no-FK table/entity/repository for idempotent state-machine apply records keyed by `(node_id, log_index)` with `(cluster_id, command_id)` reserved for replay idempotency.
- `EntryNormal` payloads now parse as tikeo command envelopes (`command_id`, `command_type`, `payload`). `noop` is applied, unknown command types are recorded as `deferred_unsupported`, invalid JSON is recorded as `rejected`, and apply index still advances deliberately.
- Next slice: choose and implement the first real Raft-owned business command, plus dynamic membership/config-change design.
- Full verification passed for this slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build`.

### 2026-05-22 Phase2 raft-rs real member command apply
- Pushed the prior 054 commit (`7f82709`) to origin/main before continuing.
- Implemented first real Raft-owned business command: `raft_member_upsert`. It validates `node_id`, http/https endpoint, and member status (`configured/joining/active/leaving/removed`), then updates the no-FK `raft_members` catalog only.
- Added command-id replay protection: `(cluster_id, command_id)` is checked before side effects, and `record_applied_command` now returns existing records for duplicate command ids to avoid unique-index failures.
- Dynamic membership remains deliberately gated: `raft_member_upsert` does not call raft-rs `propose_conf_change`; committed `EntryConfChange/EntryConfChangeV2` still stop apply progress until proposal/API + ConfState application are implemented.
- Targeted tests added for member command replay, unsupported commands, rejected payloads, invalid JSON, and raft table no-FK schema guarantees.
- Full verification passed: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs membership proposal intent API
- Added `raft_membership_proposals` as a no-FK proposal-intent table with `(cluster_id, proposal_id)` idempotency.
- Added `cluster:manage` RBAC permission for admin-only cluster mutation proposals.
- Added `POST /api/v1/raft/members:propose`; it requires a real Raft leader status, `can_schedule=true`, persisted `leader_fencing_token`, and validates add/remove voter intent before storing `pending_conf_change`.
- The endpoint deliberately does not call raft-rs `propose_conf_change` yet; committed `EntryConfChange/EntryConfChangeV2` apply remains gated until ConfState persistence and quorum-safe transition logic are implemented.
- Full verification passed for membership proposal intent API: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs committed ConfChange apply
- Added runtime command path for membership proposals: HTTP stores an idempotent proposal intent, then submits it to `RaftRuntimeCoordinator::propose_membership_change`; non-runtime/static coordinators now reject instead of pretending proposal success.
- Runtime proposals build raft-rs `ConfChange` with JSON context (`proposal_id/action/node_id/endpoint`) and call `RawNode::propose_conf_change` only after real leader/fencing checks.
- Committed `EntryConfChange` / `EntryConfChangeV2` are now explicitly decoded. With a runtime node, successful `RawNode::apply_conf_change` persists base64 `raft_metadata.conf_state`, updates `raft_members` to `active/removed`, marks proposal `applied`, and advances applied index. Without runtime node, config-change entries remain gated and do not advance.
- Malformed config-change payloads are treated as handled/rejected and advance apply index without mutating membership; unsupported multi-change V2 is rejected.
- Full verification passed for committed ConfChange apply: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs multi-node in-process E2E
- Continued `.prompt/058-phase2-raft-rs-multinode-e2e.md` after committed ConfChange apply.
- Added a deterministic in-process 3-node raft-rs `RawNode` harness that routes Ready messages directly between nodes and never fakes leadership.
- The harness now proves a real `campaign()` election can produce exactly one leader, persist `raft:term:1:node:tikeo-0`, and set `can_schedule=true` only after the token is persisted.
- Added membership proposal E2E coverage: record proposal intent, propose raft-rs ConfChange, commit/apply it, persist `raft_metadata.conf_state`, mark `raft_membership_proposals` as `applied`, and advance `raft_members` to `active` after committed apply.
- Production Ready handling now mirrors the harness by syncing HardState/log/snapshot/commit into raft-rs `MemStorage` before `advance_append`, keeping RawNode memory state aligned with DB persistence.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server raft_inprocess --all-features`; `cargo test -p tikeo-server raft --all-features`.
- Full verification passed for 058: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs restart recovery hardening
- Continued `.prompt/059-phase2-raft-rs-http-transport-e2e-or-persistence-hardening.md` after 058 push.
- Runtime startup now restores raft-rs `MemStorage` from persisted `raft_metadata` and `raft_log_entries`: HardState term/vote/commit plus stored log entries are replayed before the ticker loop starts.
- Startup now clears stale `leader_fencing_token` before runtime observation; scheduling authority must be regenerated from the current real raft-rs role instead of reused after restart.
- Added targeted test `raft_runtime_restore_replays_persisted_metadata_and_clears_stale_fencing` covering restored entries/hardstate and stale token removal.
- Next prompt `.prompt/060-phase2-raft-rs-http-transport-smoke.md` keeps the remaining HTTP/Docker bridge transport smoke as the next Phase2 slice.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server raft_runtime_restore --all-features`; `cargo test -p tikeo-server raft --all-features`.
- Full verification passed for 059: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs HTTP transport token smoke
- Continued `.prompt/060-phase2-raft-rs-http-transport-smoke.md` after 059 push.
- Added HTTP route smoke coverage for `/api/v1/raft/append-entries` with `x-tikeo-raft-token`: valid internal token bypasses human session auth and enqueues into the raft runtime inbox; invalid token falls back to normal auth and returns an unauthorized standard envelope.
- The test keeps the safety semantics explicit: `accepted=true` means local runtime queue acceptance only, local role remains follower, and no leader fencing token/scheduling authority is granted.
- Updated design roadmap to split completed route-level smoke from the remaining Docker bridge/K8s Service multi-container E2E script.
- Next prompt `.prompt/061-phase2-raft-rs-docker-bridge-e2e-script.md` targets bridge-network script verification without host networking.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server raft_append_entries_internal_token --all-features`.
- Full verification passed for 060: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs Docker bridge E2E script
- Continued `.prompt/061-phase2-raft-rs-docker-bridge-e2e-script.md` after 060 push.
- Added `scripts/raft-bridge-e2e.sh`: builds the tikeo server image, creates a Docker bridge network, starts 3 tikeo containers with generated raft configs, peers by container DNS (`tikeo-N:9090`), and injects `TIKEO__CLUSTER__TRANSPORT_TOKEN` without committing secrets.
- The script smoke-checks `/healthz`, `/api/v1/cluster`, `/api/v1/cluster/diagnostics`, and `/api/v1/raft/append-entries` through bridge networking; it also verifies wrong raft token returns 401 and that any schedulable leader is unique and has a fencing token.
- Dockerfile builder now installs `protobuf-dev gcompat` so raft-proto/protobuf build scripts work on alpine while keeping the runtime image alpine.
- Updated `config/raft.toml` peer endpoints to the actual HTTP management API port `9090`; worker tunnel remains `9998`.
- Ran `./scripts/raft-bridge-e2e.sh` successfully; output ended with `PASS: bridge-network raft HTTP smoke succeeded without host networking`.
- Full verification passed for 061: `./scripts/raft-bridge-e2e.sh`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase3 audit before/after trace result foundation
- Continued `.prompt/062-phase3-audit-before-after-trace-export.md` after Phase2 bridge E2E closeout.
- Extended `audit_logs` storage/entity/repository/API summary with `before`, `after`, `trace_id`, `result`, and `failure_reason`; SQLite compatibility adds missing columns without foreign keys.
- Existing audit helper now captures `x-request-id` / `x-trace-id` as trace id and defaults write-operation audit result to `success`; login/logout audit records also include trace/result fields.
- Audit list API exposes the new fields and the Web audit page shows result, trace id, and before/after availability.
- Added API test assertions for trace/result/failure/before/after fields in audit list output.
- Export governance is split to `.prompt/063-phase3-audit-export-governance.md` to keep row limits/redaction/content-type decisions explicit.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server audit_logs_support_server_side_filters_and_pagination --all-features`; `cargo test -p tikeo-storage migration_creates_metadata_tables --all-features`.
- Full verification passed for 062: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase3 governed audit JSON export
- Continued `.prompt/063-phase3-audit-export-governance.md` after audit trace/result foundation.
- Added `GET /api/v1/audit-logs:export` with `format=json` only, same actor/action/resource filters as list, `audit:read` permission, stable list ordering via repository, and a 500-row maximum guardrail.
- Export response keeps the standard `{ code, message, data }` envelope and includes governance metadata (`max_rows`, `redacted`, `governance`) plus exported items; CSV is rejected with a clear bad-request message until content-type/redaction rules are designed.
- Added Web audit page “导出 JSON” action that downloads the governed JSON payload for current filters.
- Updated design roadmap and created `.prompt/064-phase3-web-danger-confirm-permission-actions.md` for the next Phase3 UI governance slice.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server audit_logs_support_server_side_filters_and_pagination --all-features`; `cd web && bun run typecheck`.
- Full verification passed for 063: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase3 Web dangerous confirmations and permission-aware actions
- Continued `.prompt/064-phase3-web-danger-confirm-permission-actions.md` after governed audit export.
- Added `web/src/components/Permission.tsx` with `useCan`, `PermissionGate`, and `GuardedButton` for RBAC-aware action hiding/disable and optional `Popconfirm` gating.
- Jobs page now hides trigger actions without `instances:execute` and hides create action without `jobs:write`.
- Users page now gates create/edit/delete behind `users:manage` and delete uses destructive confirmation.
- Scripts page now gates create/edit/status transitions/delete behind `scripts:manage`; destructive or lifecycle-changing transitions require confirmation.
- Workflows page now gates create/edit behind `workflows:manage`, run/manual materialize/advance/retry behind `workflows:execute`, and dangerous runtime mutations require confirmation.
- Targeted verification so far: `cd web && bun run typecheck`.
- Full verification passed for 064: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun run build` (Vite chunk-size warning only).


### 2026-05-22 Phase3 route meta, lazy loading, 401/403, URL query governance
- Continued `.prompt/065-phase3-route-meta-lazy-401-403-url-governance.md` after Web dangerous action governance.
- Added `web/src/routes.tsx` as route/menu/permission metadata source and rewired `AppShell` + route guards to consume it.
- Converted routed pages to React lazy chunks with a shared `RouteFallback` without removing permission guards.
- Added API auth error hooks: 401 clears token and redirects to login; 403 routes to a unified forbidden page while preserving envelope handling.
- Added `useUrlQueryState` and persisted list filters/page state for audit logs, jobs, scripts, and workflows.
- Added API client tests for 401/403 auth-error behavior.
- Targeted verification so far: `cd web && bun run typecheck`; `cd web && bun test`.
- Full verification passed for 065: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build` (Vite chunk-size warning remains for Scripts/main chunks).

### 2026-05-22 Phase3 WASM sandbox processor boundary
- Started `.prompt/066-phase3-wasm-sandbox-processor-spike.md` after Web route/auth governance.
- Checked current crates.io via cargo: `wasmtime = "45.0.0"`; upstream Wasmtime docs expose fuel/epoch interruption and Store resource limiter APIs suitable for worker-side limits.
- Added `tikeo-core` WASM contract types: `WasmRuntimeKind`, `WasmCapabilities`, `WasmResourcePolicy`, `WasmProcessorSpec`, and `WasmSpecError`.
- Default WASM processor spec selects Wasmtime, `_start`, 30s timeout, 64MiB memory, fuel budget, no network, no preopened host directories, and validates denial of ambient host access.
- Added core tests for stable wire serialization and policy validation.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-core --all-features`.
- Full verification passed for 066: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build` (Vite chunk-size warning unchanged).

### 2026-05-22 Phase3 WASM worker runtime executor
- Started `.prompt/067-phase3-wasm-worker-runtime-executor.md`.
- Added dedicated `crates/tikeo-wasm` so Wasmtime remains worker/runtime-boundary only and is not pulled into server HTTP/storage paths.
- Implemented `WasmExecutor` on Wasmtime 45.0.0 with fuel metering, epoch interruption timeout hook, memory cap via StoreLimits, and no WASI ambient imports.
- Added tests for minimal WAT execution, network-capability rejection, missing entrypoint rejection, and fuel exhaustion on a busy loop.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-wasm --all-features`; `cargo clippy -p tikeo-wasm --all-targets --all-features -- -D warnings`.
- Full verification passed for 067: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build` (Vite chunk-size warning unchanged).

### 2026-05-22 Phase3 WASM script binding and dispatch metadata
- Started `.prompt/068-phase3-wasm-script-binding-and-dispatch.md`.
- Extended worker proto in server/Rust SDK/Java SDK with `DispatchTask.processor_binding`, `TaskProcessorBinding`, and `WasmProcessorBinding` for dynamic WASM payload + policy metadata.
- Dispatcher now receives `ScriptRepository`; when `processor_name` is `script:<id>`, it loads the script and attaches WASM binding only when `language=wasm`, `status=approved`, and `WasmProcessorSpec` validates default-deny network/filesystem policy.
- Server still does not execute user code; it only passes approved module bytes and policy metadata to connected workers.
- Added dispatcher tests for approved safe WASM binding shape and rejection of draft / network-enabled WASM scripts.
- Rust Worker SDK proto fixture updated and SDK tests passed after regenerated proto.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server tunnel::dispatcher --all-features`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features`.
- Java SDK Gradle test was attempted but first Gradle distribution download was too slow and was stopped; rerun once Gradle is cached.
- Full verification passed for 068: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features`. Java SDK Gradle test was attempted but not completed because the first Gradle distribution download was too slow; rerun once cached.
- Re-ran final 068 verification after proto boxing/clippy fixes: all listed Rust/backend/web/Rust-SDK checks passed again. Java SDK Gradle remains not completed due slow first distribution download.

### 2026-05-22 Java SDK Gradle verification补齐
- Updated `sdks/java/gradlew` to default to Gradle 9.5.1 (latest stable confirmed from Gradle releases) and use a faster configurable distribution URL, defaulting to Huawei Cloud Gradle mirror while allowing `GRADLE_DISTRIBUTION_URL` override.
- Fixed Java Gradle wrapper project root so `./gradlew test` runs from `sdks/java` instead of repo root.
- Java SDK verification passed: `cd sdks/java && ./gradlew --version --no-daemon` (Gradle 9.5.1) and `cd sdks/java && ./gradlew test --no-daemon` (BUILD SUCCESSFUL, 18 tasks executed). Gradle reports deprecated features warning for Gradle 10 compatibility follow-up.


### 2026-05-22 Phase3 WASM SDK execution adapters
- Continued `.prompt/069-phase3-wasm-sdk-execution-adapters.md` after Java Gradle verification补齐.
- Rust Worker SDK now detects `DispatchTask.processor_binding.wasm`; normal SDK processors remain unchanged for regular tasks.
- Added opt-in Rust SDK `wasm` feature with Wasmtime 45.0.0 adapter, fuel metering, epoch timeout, memory limit, default network rejection, and tests for enabled execution / network rejection / disabled-feature failure.
- Java core SDK now explicitly reports unsupported WASM processor binding and does not invoke the user `TaskProcessor` for WASM-bound dispatches.
- Updated design roadmap and created `.prompt/070-phase3-wasm-distribution-integrity-and-gradle10-cleanup.md`.
- Full verification passed for 069: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --no-daemon`.
- Known warning: Java Gradle build still reports deprecated features that need Gradle 10 compatibility cleanup.


### 2026-05-22 Phase3 WASM distribution integrity and Gradle 10 cleanup
- Continued `.prompt/070-phase3-wasm-distribution-integrity-and-gradle10-cleanup.md`.
- Extended Worker Tunnel `WasmProcessorBinding` with immutable version hooks (`version_id`, `version_number`), `module_sha256`, and reserved `module_signature` across server proto, Rust SDK proto, and Java SDK proto.
- Script version snapshots now persist `content_sha256`; `ScriptSummary` computes SHA-256 for the current script content without adding database foreign keys.
- Dispatcher includes SHA-256 in WASM bindings and uses matching immutable script version snapshot metadata when available; otherwise it still sends digest-only integrity metadata.
- Rust Worker SDK validates `module_sha256` before Wasmtime compilation/execution and fails digest mismatches clearly.
- Web script management now shows content SHA-256 and WASM sandbox defaults/policy metadata in list/detail/version views.
- Java Gradle protobuf plugin upgraded to 0.10.0 and protoc/grpc artifacts use explicit platform classifier notation, removing Gradle 10 multi-string dependency deprecation warnings under Gradle 9.5.1 `--warning-mode all`.
- Full verification passed for 070: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-22 Phase3 script release pointer and worker version binding
- Continued `.prompt/071-phase3-script-release-pointer-and-worker-version-binding.md` after WASM distribution integrity.
- Added `scripts.released_version_id` / `released_version_number` as soft release pointers to immutable `script_versions` snapshots; no database foreign keys were introduced.
- Fixed script version creation to handle empty version history safely and return constructed summaries without SQLite NULL aggregate decode failures.
- Added repository publish/rollback APIs that move the release pointer and keep current draft content mutable but non-executable.
- Added HTTP `POST /api/v1/scripts/{id}/publish` and `/rollback` endpoints using standard `{code,message,data}` envelopes and audit actions `publish`/`rollback`.
- Dispatcher now fails closed for approved WASM scripts without a release pointer or missing released version, and worker bindings use released snapshot bytes, SHA-256, version id, and version number.
- Web script page now shows released version/id, marks released history rows, and exposes publish/rollback actions under script manage permission.
- Full verification passed for 071: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.
- Known warning: Web build still reports existing >500KB chunk-size warning for large lazily loaded chunks.

### 2026-05-22 Phase3 script policy metadata, runner abstraction, and Web chunk split
- Continued `.prompt/072-phase3-script-policy-engine-and-sandbox-runners.md` and the user-requested Web chunk optimization.
- Added `ScriptExecutionPolicy` in core with resources/network/filesystem/secrets/env metadata and default-deny validation for dangerous grants.
- Persisted policy snapshots on `scripts.policy_json` and immutable `script_versions.policy_json`; compatibility migration uses soft schema changes only and still no database foreign keys.
- HTTP script create/update accepts optional `policy`, rejects network/filesystem/secret grants for now, and returns policy data in the standard envelope.
- Script version diff now includes `policy` changes; Web script management exposes safe resource/env policy fields and policy summaries.
- Rust Worker SDK now has non-WASM `ScriptRunnerKind`, `ScriptRunnerPolicy`, `ScriptRunnerTask`, `ScriptRunner` and `UnsupportedScriptRunner` abstraction; unsupported runner validates default-deny policy and refuses execution until concrete sandbox runners are implemented.
- Web build chunk issue fixed with Vite/Rolldown `codeSplitting.groups` for React/AntD/CodeMirror/utility vendor chunks; `bun run build` no longer emits >500KB chunk warnings.
- Full verification passed for 072: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-22 Phase3 local subprocess script runner foundation
- Continued `.prompt/073-phase3-script-sandbox-runner-implementations.md`.
- Added Rust SDK `LocalSubprocessScriptRunner` as the first opt-in non-WASM runner boundary for Shell/Python/Node/PowerShell/Rhai command mappings.
- Runner validates default-deny policy, requires released immutable version metadata, verifies content SHA-256 before execution, clears inherited env, only forwards whitelisted env vars plus tikeo metadata, feeds script through stdin, enforces wall-clock timeout, and caps captured stdout+stderr bytes.
- Added SDK tests for successful shell execution, digest mismatch, missing released snapshot, timeout, output limit, and missing runtime.
- Full verification passed for 073 slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-22 Phase3 non-WASM script runner protocol and UI binding
- Continued `.prompt/074-script-runner-protocol-and-ui-binding.md` after the local subprocess runner foundation.
- Extended Worker Tunnel protocol with `ScriptProcessorBinding` for Shell/Python/Node/PowerShell/Rhai released snapshot payloads while preserving WASM bindings.
- Dispatcher now fails closed unless a script is approved, has a release pointer, resolves to an immutable released `script_versions` row, and the released snapshot itself passes default-deny policy validation.
- Worker selection now honors unified dynamic script capability `script`; legacy `script:wasm`, `script:<language>`, `script:*`, and `*` remain compatible for controlled or older workers.
- Rust Worker SDK added `ScriptRunnerRegistry` and executes non-WASM bindings only when the worker explicitly registers a matching runner; missing runners produce a clear failure result.
- Java SDK now explicitly reports unsupported script processor bindings and does not call the normal task processor for them.
- Web script detail drawer now documents required worker capabilities and runtime support for WASM and non-WASM scripts.
- Full verification passed for 074: `cargo fmt --all -- --check`; `cargo test -p tikeo-proto --all-features`; `cargo test -p tikeo-server --all-features tunnel::dispatcher -- --nocapture`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd web && bun run typecheck`; `cd web && bun test && bun run build`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-22 Phase3 containerized script runner foundation
- Continued `.prompt/075-script-runner-container-and-execution-governance.md` after non-WASM Worker Tunnel protocol binding.
- Added Rust SDK `ContainerScriptRunner` as an explicit Worker-side opt-in runner for non-WASM dynamic scripts.
- Refactored the Rust Worker SDK away from a monolithic `lib.rs`: `lib.rs` now only declares/re-exports modules; implementation moved into `config`, `session`, `task`, `error`, `script`, `wasm`, `proto`, and tests modules, with script runners split into `script/local.rs` and `script/container.rs`.
- The container runner builds Docker-compatible `run --rm -i` commands, passes released script content via stdin, disables container networking with `--network=none`, uses `--read-only`, mounts no host paths, injects tikeo metadata env, and forwards only policy-whitelisted env vars.
- Shared released snapshot validation between local subprocess and container runners: language match, version_id/version_number, content SHA-256, default-deny policy, and dangerous network/filesystem/secret rejection before spawn.
- Added deterministic unit tests for container command boundary and pre-runtime dangerous policy rejection; live Docker/K8s smoke and audit/result governance move to 076.
- Full verification passed for 075 after SDK module split: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd web && bun run typecheck && bun test && bun run build`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-22 Project rename to tikeo
- Renamed project identity from the previous project identity to tikeo across Rust workspace package/crate names, binary name, Docker/Compose/K8s identifiers, config defaults, scripts, docs, memory, and prompts.
- Renamed Rust SDK to `tikeo` and Java SDK modules to `tikeo`, `tikeo-spring`, and `tikeo-spring-boot-starter`.
- Changed Java package prefix to `net.tikeo` and updated example imports/application main class.
- Changed worker protobuf package namespace to `tikeo.worker.v1` and updated Rust/Java generated-code references.
- Prepared `.prompt/077-script-execution-governance-after-tikeo-rename.md` as the next handoff prompt.
- Targeted verification so far: `cargo check --workspace --all-features`; `cargo fmt --all`.

### 2026-05-22 SDK naming contraction
- Applied user-requested SDK naming contraction: Rust SDK previous Rust Worker SDK name -> `tikeo`, Java core SDK module previous Java core SDK name -> `tikeo`.
- Updated Rust example dependency/imports to use `tikeo = { path = "../../../sdks/rust/tikeo" }`.
- Updated Java Gradle composite build so `tikeo-spring` depends on `project(":tikeo")`; Java package prefix remains `net.tikeo`.
- Rename verification fixed one regression: the default admin password text changed to `<retired-password>`, so the seeded BCrypt hash was regenerated to match the new credential.
- Full rename verification passed: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.
### 2026-05-23 Phase3 script execution governance visibility
- Continued `.prompt/077-script-execution-governance-after-tikeo-rename.md`.
- Added dispatcher-side script governance instance logs for fail-closed dispatch cases and worker capability misses: missing script, not approved, missing release pointer/version, unsupported language, policy rejection, and no eligible unified `script` worker capability.
- Added Rust SDK script failure classification via `TaskOutcome::failure_class()` and JSON result messages for missing runner, policy rejection, digest mismatch, timeout, output limit, and runtime unavailable; Server persists these Worker result classes as `script_execution_governance` instance logs.
- Documented script-capable Worker Pool deployment for Docker/K8s and `ContainerScriptRunner` opt-in constraints in design and Rust SDK README.
- Updated design roadmap and created `.prompt/078-script-governance-audit-alerting.md` for first-class audit/alert follow-up.
### 2026-05-23 Phase3 script governance query/UI/alert foundation
- Continued `.prompt/078-script-governance-audit-alerting.md`.
- Instance log API now parses `script_execution_governance` JSON logs into `governance_event`, `governance_failure_class`, and `governance_message` fields while preserving plain log compatibility.
- Added compatibility filtering for governance logs via `/api/v1/instances/{id}/logs?page_token=script_execution_governance`.
- Web Instances log drawer now highlights script governance failures and summarizes governance event count.
- Added `AlertCondition::ScriptGovernanceFailure` as the deterministic alert rule shape for follow-up alert materialization.
- Updated design roadmap and created `.prompt/079-script-governance-audit-materialization.md`.

### 2026-05-23 — Workflow legacy edge condition frontend normalization
- Fixed the dev workflow 400 path where `wf-dev-basic-pipeline` carried legacy `condition: "success"` from seed data even though the visible Web selector uses canonical `on_success`.
- Web workflow client now normalizes legacy aliases before workflow create/update/dry-run, and the editor normalizes loaded definitions before showing the JSON draft.
- `scripts/dev-seed.sql` now seeds the sample workflow definition and workflow_edges row with canonical `on_success`.
- Regression tests cover stale `success`/`failed` aliases escaping through update/dry-run serialization.

### 2026-05-23 — Script edit UX and workflow canvas fullscreen
- Script edit moved from modal to guarded `/scripts/:id/edit` secondary page with a wide CodeMirror-centered layout and side cards for metadata/runtime/policy.
- Script update still requires diff preview confirmation before saving and still creates immutable version snapshots through the existing API.
- Workflow visual editor canvas now supports fullscreen toggle and Escape exit for large DAG editing.

### 2026-05-23 — Script editor back button compact style
- Script editor secondary page back button now keeps natural width instead of stretching across the hero content column.


### 2026-05-23 — Phase 3 / Phase 4 roadmap scope adjustment
- Moved Node.js SDK, K8s Helm Chart, and PowerJob migration tooling out of Phase 3 into Phase 4.
- Added XXL-JOB migration tooling to Phase 4 alongside PowerJob migration tooling.
- Phase 3 remains focused on enterprise governance/runtime safety work such as script governance, policy, audit, alerts, metrics, tracing, and Java/Rust SDK maturity.
### 2026-05-23 — Phase 079 script governance audit materialization
- Continued `.prompt/079-script-governance-audit-materialization.md`.
- Added `tunnel::governance` helper so dispatcher-side fail-closed script governance events and Worker result failure classes share the same canonical `script_execution_governance` payload.
- Materialized governance failures into durable `audit_logs` rows with `action=script_governance_failure`, `resource_type=script_execution_governance`, soft `resource_id=<instance_id>`, `result=failed`, and `failure_reason=<failure_class>`; no database foreign keys were added.
- Added audit repository/API filtering by `failure_reason`; Web audit page now keeps the filter in URL state and export uses the same filter.
- Added regression coverage proving governance audit rows can be queried by failure class.
### 2026-05-23 — Phase 080 alert rule API and event history
- Continued `.prompt/080-alert-rule-event-history.md`.
- Added persistent `alert_rules` and `alert_events` metadata tables plus repository support for rule creation/listing and event history queries.
- Exposed `/api/v1/alert-rules` and `/api/v1/alert-events` HTTP APIs behind existing admin/audit permissions and kept responses in the standard `{ code, message, data }` envelope.
- Wired script governance materialization to append alert history entries alongside audit rows, including basic threshold/dedupe/silence handling for `script_governance_failure` rules.
- Added regression coverage for alert rule creation, governance event ingestion, and alert event history queries.
### 2026-05-23 — Phase 081 alert recovery and notification history
- Continued `.prompt/081-alert-recovery-and-notifications.md`.
- Added deterministic alert recovery transitions by appending `script_governance_recovery` history rows with `status=recovered` instead of mutating prior events.
- Exposed a recovery HTTP endpoint for alert events and kept list/history queries stable for operators.
- Added regression coverage proving a firing governance alert can be resolved into a recovery history entry while preserving the original firing event.
### 2026-05-23 — Phase 082 alert notification summary
- Continued `.prompt/082-alert-notification-summary.md`.
- Added `/api/v1/alert-events:summary` to roll up alert event history by rule, resource, and failure class while preserving list filters.
- Summary rows include latest status/type/message, first/last seen timestamps, and firing/suppressed/silenced/recovered counts for operator notification history review.
- Added regression coverage proving firing, suppressed, and recovered history rows collapse into a single deterministic summary without external webhook smoke.
### 2026-05-23 — Phase 083 metrics summary and SLO API
- Continued `.prompt/083-metrics-summary-and-slo.md`.
- Added deterministic `/api/v1/metrics/summary` for operator dashboards without requiring external Prometheus/Grafana services in tests.
- Summary includes online worker count, job instance status counts, alert event status counts, and script governance failure counts by failure class.
- Added targeted regression coverage for storage/registry/alert count aggregation and standard HTTP envelope behavior.
### 2026-05-23 — Phase 084 OpenTelemetry tracing foundation
- Continued `.prompt/084-opentelemetry-tracing-foundation.md`.
- Added HTTP trace-id middleware that accepts `x-request-id`, `x-trace-id`, or W3C `traceparent`, generates `trc-*` when missing, and writes `x-trace-id` on API responses.
- Added local tracing span fields for method/path/trace_id without requiring an OTLP collector in tests; audit helpers now resolve the same generated/propagated trace id from request headers.
- Added targeted regression coverage for explicit and generated trace-id response behavior plus traceparent parsing.
### 2026-05-23 — Phase 085 OIDC/SSO foundation
- Continued `.prompt/085-oidc-sso-foundation.md`.
- Added `auth` / `auth.oidc` configuration shapes with local login enabled and OIDC disabled by default.
- Added public `GET /api/v1/auth/status` so clients can distinguish local vs OIDC-ready auth mode and see redacted provider metadata without a live IdP.
- Preserved existing local admin login/session/RBAC behavior and added regression coverage for local and configured OIDC status responses.
### 2026-05-23 — Phase 086 mTLS transport foundation
- Continued `.prompt/086-mtls-transport-foundation.md`.
- Added `transport_security` configuration shapes for HTTP and Worker Tunnel TLS/mTLS while keeping local development plaintext by default.
- Added `GET /api/v1/security/transport` diagnostics that redacts paths but reports TLS/mTLS readiness and partial-config issues.
- Added targeted regression coverage for default plaintext readiness and partial Worker Tunnel mTLS configuration diagnostics without certificate/network smoke.
### 2026-05-23 — Phase 087 script approval policy gates
- Continued `.prompt/087-script-approval-policy-gates.md`.
- Added publish/rollback policy gate checks that re-validate immutable script version snapshots before changing the release pointer.
- Dangerous legacy/imported versions that request network/filesystem/secret grants are blocked with a standard bad-request envelope and `failure_reason=script_policy_approval_required` audit rows.
- Safe script publish and rollback behavior remains unchanged; added regression coverage for blocked dangerous publish/rollback and queryable failed audit entries.

### 2026-05-23 — Phase 088 Grafana dashboard template foundation
- Continued `.prompt/088-phase3-remaining-hardening.md` with the smallest locally verifiable observability hardening slice.
- Added `observability/grafana/tikeo-phase3-dashboard.json` as a deterministic Grafana dashboard template for existing Prometheus metrics: HTTP request rate, HTTP p95 latency, connected workers, worker dispatch outcomes, and an HTTP error-ratio SLO placeholder.
- Added a Rust integration test that parses the dashboard JSON, asserts the expected title/panels shape, and verifies the required metric query strings stay present without needing a live Grafana/Prometheus service.
- Roadmap now marks the Grafana template foundation complete while leaving richer scheduling-latency/business SLO metrics open.

### 2026-05-23 — Phase 089 dispatch queue SLO summary
- Continued `.prompt/089-phase3-business-slo-metrics.md` with a locally verifiable business SLO metric slice.
- Added `DispatchQueueSloSummary` over existing `dispatch_queue` rows: total/by_status, pending/running counts, oldest pending age seconds, and average pending age seconds.
- Extended `GET /api/v1/metrics/summary` with the queue SLO summary while preserving the standard HTTP envelope and avoiding external Prometheus/Grafana dependencies in tests.
- Updated the Grafana template with a dispatch queue pending-age SLO query placeholder and kept JSON/metric-reference validation deterministic.

### 2026-05-23 — Phase 090 OTLP exporter status foundation
- Continued `.prompt/090-phase3-otel-exporter-foundation.md` with configuration/readiness plumbing before adding network exporter side effects.
- Added `observability.tracing` config with disabled-by-default OTLP export, optional endpoint, and header-name metadata; local dev and container configs keep export disabled.
- Added `GET /api/v1/observability/status` behind `system:read` permission to report tracing exporter readiness while redacting endpoint values and header secrets.
- Added regression coverage for default no-collector mode and configured OTLP readiness without requiring a live collector.

### 2026-05-23 — Phase 091 alert delivery readiness foundation
- Continued `.prompt/091-phase3-alert-provider-delivery-foundation.md` with a no-network notification hardening slice.
- Added `GET /api/v1/alert-rules/{id}/delivery-status` behind existing audit read permission.
- Delivery status parses persisted rule channels, reports provider/target/secret readiness, and redacts URLs/tokens/secrets from the response.
- Added regression coverage for webhook/email channel readiness and redaction without sending real external notifications.

### 2026-05-23 — Phase 092 OIDC authorize/callback skeleton
- Continued `.prompt/092-phase3-oidc-callback-skeleton.md` with a no-IdP local SSO shape slice.
- Added `GET /api/v1/auth/oidc/authorize` to build a redacted authorization URL from configured issuer/client/scopes without contacting the provider.
- Added `GET /api/v1/auth/oidc/callback` as a safe callback contract that validates code/state shape but refuses to create sessions until real token exchange/external identity mapping exists.
- Added regression coverage for disabled default behavior, configured authorize URL shape, secret redaction, and callback fail-closed behavior.

### 2026-05-23 — Phase 093 script approval/signature fail-closed skeleton
- Continued `.prompt/093-phase3-script-approval-signature-skeleton.md` with release metadata gates instead of runtime behavior.
- Added `approval_ticket` and `signature` fields to `ScriptReleaseRequest` so clients cannot silently send ignored approval/signature data.
- Publish/rollback now fail closed when those fields are present until real signature verification exists, and materialize `failure_reason=script_signature_verification_required` audit rows.
- Existing safe publish/rollback and dangerous policy gate behavior remain unchanged; Server still never executes user code.

### 2026-05-23 — Phase 094 transport listener boundary
- Continued `.prompt/094-phase3-transport-listener-boundary.md` with a fail-closed TLS readiness boundary.
- `GET /api/v1/security/transport` now reports `listener_mode` per endpoint: plaintext by default, `tls_pending_listener` when TLS is configured but listener wiring is not implemented.
- TLS/mTLS-enabled configs are no longer considered ready solely because cert/key paths are present; status issues explicitly call out pending listener wiring while keeping paths redacted.
- Added regression coverage for default plaintext, partial mTLS config, and fully path-configured HTTP TLS still failing closed until real TLS serving exists.

### 2026-05-23 — Phase 3 closeout review
- Completed `.prompt/095-phase3-closeout-review.md` as an honest roadmap closeout pass after the Phase 088-094 hardening run.
- Confirmed Phase 3 top-level items that still require external systems or larger production wiring remain unchecked: real OIDC token exchange and external identity mapping, real TLS/mTLS listeners, full script approval/signing/grants, real alert provider delivery, complete business SLO metrics, and real OTLP exporter smoke.
- Added Phase 3 closeout notes to `design/tikeo-architecture-design.md` summarizing completed local foundations vs remaining production gaps.
- Deferred Phase 4 scope remains unchanged: Node.js SDK, K8s Helm, PowerJob migration tooling, and XXL-JOB migration tooling.

### 2026-05-23 — Phase 096 dispatch queue Prometheus SLO metric
- Continued Phase 3 observability hardening by making the Grafana dispatch queue pending-age query backed by a real server-emitted Prometheus histogram instead of a dashboard-only placeholder.
- `GET /api/v1/metrics/summary` now records `tikeo_dispatch_queue_pending_age_seconds{stat="oldest|average"}` and `tikeo_dispatch_queue_items_total{status="pending|running"}` into the same local Prometheus recorder exposed by `/metrics`.
- Added regression coverage that calls the summary endpoint, then scrapes `/metrics` and asserts the dispatch queue pending-age metric is present.
- Full business SLO coverage remains open for broader dispatch latency, instance success-rate, workflow SLA, and map-reduce metrics.

### 2026-05-23 — Phase 097 business SLO Prometheus snapshots
- Continued `.prompt/097-phase3-business-slo-prometheus-snapshots.md` by promoting more existing metrics summary data into real Prometheus series.
- `GET /api/v1/metrics/summary` now records worker online, job instance status, job instance success ratio, alert status, and script governance failure gauges into the router-local recorder exposed by `/metrics`.
- Extended regression coverage so the summary-then-scrape path proves the new instance success and script governance SLO metric names are emitted.
- Kept full business SLO coverage open for end-to-end dispatch latency histograms, workflow/map-reduce SLA, and live Prometheus recording-rule validation.

### 2026-05-23 — Phase 098 API token lifecycle foundation
- Continued Phase 3 auth/RBAC hardening with a durable API token lifecycle slice.
- Added authenticated `POST /api/v1/auth/api-tokens`, `GET /api/v1/auth/api-tokens`, and `DELETE /api/v1/auth/api-tokens/{id}` endpoints.
- API tokens reuse the DB-backed session store, persist only SHA-256 token hashes, return the raw bearer token only at creation time, hide `token_hash` from list responses, and invalidate bearer access immediately on revoke.
- Added audit entries for API token create/revoke; fine-grained token scopes, rotation policy, and multi-tenant scope binding remain future work.

### 2026-05-23 — Phase 099 scoped API token permissions
- Continued `.prompt/099-phase3-api-token-scopes.md` by adding fine-grained API token scope allow-lists.
- `POST /api/v1/auth/api-tokens` now accepts optional `scopes` in `resource:action` form, validates every requested scope against the current principal permissions, stores the scope metadata with the hashed token session, and returns scopes in token metadata.
- Scoped API tokens now resolve to narrowed effective permissions; an `admin` role no longer bypasses scoped-token limits, so a `users:read` token can list users but cannot create users.
- Multi-tenant namespace/app/worker-pool scope binding and token rotation/expiry policy remain future work.

### 2026-05-23 — Phase 100 Worker cluster page interaction/layout refresh
- Responded to the user-requested Worker cluster page UX fix by replacing the two-list layout with an operations dashboard.
- Split `WorkersPage` into focused components under `web/src/pages/workers/`: cluster overview/queue stats, filterable worker table, dispatch queue status panel, and pure page-model helpers.
- Added worker search, namespace filtering, capability filtering, queue status drill-down, queue pressure/health affordances, responsive CSS, and static regression coverage for the new interaction contracts.
- No API contract changes; the page still uses `GET /api/v1/workers` and `GET /api/v1/dispatch-queue`.

### 2026-05-23 — Phase 101 Java Spring worker demo runtime fix
- Reproduced the Java Spring worker demo exiting immediately: `DemoRunner` called `client.close()` directly after `client.start()`, and the README command used the SDK wrapper without selecting the demo project.
- Updated the demo runner to stay alive on a `CountDownLatch` until shutdown and close the worker client from `@PreDestroy`; dry-run bootRun now remains running instead of exiting immediately.
- Changed the demo default Worker Tunnel endpoint to `http://127.0.0.1:9998`, added/committed a local demo `gradlew`, ignored demo `.gradle/`, and fixed README/root verification commands.
- Started tikeo with `config/dev.toml`, started the Java demo with `TIKEO_WORKER_DRY_RUN=false`, and verified `/api/v1/workers` reports one online worker with `java` and `spring-boot` capabilities.
### 2026-05-24 — Phase 102 API token expiry and rotation policy
- Continued `.prompt/102-phase3-api-token-expiry-rotation.md` with the remaining API token governance gap from Phase 3.
- Added `auth.api_tokens` policy defaults for token default/min/max TTL and exposed the dev config values explicitly.
- `POST /api/v1/auth/api-tokens` now accepts bounded `expires_in_seconds`; out-of-policy TTL requests fail with a standard bad-request envelope.
- Added `POST /api/v1/auth/api-tokens/{id}/rotate` to preserve existing scopes, issue a replacement token, revoke the old token immediately, and audit the rotation.
- Multi-tenant namespace/app/worker-pool scope binding remains open.
Verification evidence:
- `rtk cargo test -p tikeo-server api_token_policy --all-features` failed before implementation for ignored TTL and missing TTL bound rejection, then passed after implementation.
- `rtk cargo test -p tikeo-server api_token --all-features` passed.
- `rtk cargo test -p tikeo-config default_auth_config --all-features` passed.
- `rtk cargo fmt --all -- --check` passed.
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `rtk cargo test --workspace --all-features` passed: 120 tests.
- `rtk cargo build --workspace --all-features` passed.
- `rtk cargo run -- --help` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm` passed.
- `rtk cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` passed.
- `rtk bash -lc 'cd web && bun run lint'` passed.
- `rtk bash -lc 'cd web && bun run typecheck && bun test && bun run build'` passed.
- `rtk bash -lc 'cd sdks/java && ./gradlew test --warning-mode all --no-daemon'` passed.
- Runtime smoke on temporary 127.0.0.1:19090/19998 server verified healthz, login, scoped token creation with 900s TTL, rotation, old-token 401, and new-token scoped users read.
### 2026-05-24 — Web login bypass and root dashboard route
- Responded to user UX feedback that `/login` should not stay visible while a session token exists and the bare domain should have a default page.
- Added an explicit `/` route redirecting to `ROUTE_META.dashboard.path`, so direct domain access lands on the overview route before protected-route auth handling.
- `LoginPage` now checks `getAuthToken()` on mount and replace-navigates to the dashboard when a token is present; successful login still returns to the originally requested protected path when available.
- Added a source-level route regression test for the login bypass and root default route.
- Verification passed: targeted RED/green route test and full Web `lint`, `typecheck`, `bun test`, `build`.
### 2026-05-24 — Phase 104 API token namespace/app/worker-pool scope bindings
- Continued `.prompt/104-phase3-api-token-scope-bindings.md` by closing the remaining API-token multi-tenant binding foundation gap.
- Added `AccessScopeBinding` metadata with optional namespace/app/worker_pool fields; API token create/list, rotate, and `/auth/me` now preserve and expose bindings without plaintext token storage.
- Enforced namespace/app bindings for job list/create/trigger: bound tokens only see matching jobs and cannot create/trigger outside their binding.
- Enforced worker-pool visibility for `/api/v1/workers` using `worker_pool` / `worker-pool` worker labels in addition to namespace/app.
- Full tenant/app/worker-pool CRUD/UI and OIDC identity-to-tenant mapping remain open.
Verification evidence:
- `rtk cargo test -p tikeo-server api_token_scope_bindings --all-features` failed before implementation because bindings were ignored, then passed with 2 tests.
- `rtk cargo test -p tikeo-server api_token --all-features` passed with 6 tests.
- `rtk cargo fmt --all -- --check` passed.
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `rtk cargo test --workspace --all-features` passed: 122 tests.
- `rtk cargo build --workspace --all-features` passed.
- `rtk cargo run -- --help` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm` passed.
- `rtk cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` passed.
- `rtk bash -lc 'cd web && bun run lint && bun run typecheck && bun test && bun run build'` passed.
- `rtk bash -lc 'cd sdks/java && ./gradlew test --warning-mode all --no-daemon'` passed.

### 2026-05-24 — Phase 105 alert webhook delivery foundation
- Continued `.prompt/105-phase3-alert-webhook-delivery.md` by converting alert webhook delivery from log-only skeleton into real HTTP POST delivery with structured results.
- Added `AlertDeliveryPolicy` with production-safe default HTTPS/public-only validation and explicit loopback-HTTP allowance for local smoke tests.
- `AlertDispatcher` now returns per-channel delivery results with provider, redacted target, accepted status, HTTP status, and error details; email remains explicitly unsupported.
- Script governance alert materialization now returns created alert events and delivers notification channels for newly firing events.
- Non-webhook providers, retries/DLQ, and persisted delivery attempt history remain future provider-delivery work.
Verification evidence:
- `rtk cargo test -p tikeo-server production_policy_rejects_insecure_loopback_webhook --all-features` passed.
- `rtk cargo test -p tikeo-server webhook_dispatch_posts_payload_to_allowed_local_receiver --all-features` passed.
- `rtk cargo test -p tikeo-server alert --all-features` passed.
- `rtk cargo fmt --all -- --check` passed.
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `rtk cargo test --workspace --all-features` passed: 124 tests across workspace suites.
- `rtk cargo build --workspace --all-features` passed.
- `rtk cargo run -- --help` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm` passed.
- `rtk cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` passed.
- `rtk bash -lc 'cd web && bun run lint && bun run typecheck && bun test && bun run build'` passed.
- `rtk bash -lc 'cd sdks/java && ./gradlew test --warning-mode all --no-daemon'` passed.

### 2026-05-24 — Phase 106 workflow and map-shard SLO metrics
- Continued `.prompt/106-phase3-workflow-slo-metrics.md` by adding workflow SLA coverage to the metrics summary and Prometheus recorder path.
- `GET /api/v1/metrics/summary` now includes workflow instance totals/statuses, terminal success ratio, duration rollups, shard totals/statuses, shard success ratio, and shard duration rollups.
- `/metrics` now exposes workflow instance/shard status gauges, success-ratio gauges, and `tikeo_workflow_instance_duration_seconds` / `tikeo_workflow_shard_duration_seconds` histograms.
- Updated the Phase 3 Grafana dashboard template with real workflow SLA queries.
- Remaining observability gaps are end-to-end dispatch latency histograms, live Prometheus/Grafana recording-rule validation, and real OTLP collector smoke.
Verification evidence:
- `rtk cargo test -p tikeo-server metrics_summary_reports_storage_registry_and_alert_counts --all-features` failed before implementation because `data.workflows` was missing, then passed.
- `rtk cargo test -p tikeo-server --test grafana_dashboard --all-features` passed.
- `rtk cargo fmt --all -- --check` passed.
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `rtk cargo test --workspace --all-features` passed: 124 tests across workspace suites.
- `rtk cargo build --workspace --all-features` passed.
- `rtk cargo run -- --help` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm` passed.
- `rtk cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` passed.
- `rtk bash -lc 'cd web && bun run lint && bun run typecheck && bun test && bun run build'` passed.
- `rtk bash -lc 'cd sdks/java && ./gradlew test --warning-mode all --no-daemon'` passed.

### 2026-05-24 — Phase 107 alert provider delivery adapters
- Continued `.prompt/107-phase3-alert-non-webhook-providers.md` by adding provider-specific alert channel adapters beyond generic webhooks.
- `NotificationChannel` now supports Slack, DingTalk, Feishu/Lark, WeChat Work/WeCom, and PagerDuty channel variants with provider-specific JSON payloads.
- Provider delivery reuses the webhook delivery safety policy: default production mode remains HTTPS/public-only, and loopback HTTP is only available through explicit local policy.
- Added local Axum receiver coverage that verifies all provider adapters POST and emit expected payload shapes without contacting external services.
- Email/SMTP, retries/backoff/DLQ, persisted delivery attempts, and live provider smoke remain future hardening.
Verification evidence:
- `rtk cargo test -p tikeo-server provider_dispatch_posts_expected_payload_shapes_to_allowed_local_receivers --all-features` failed before implementation because provider channel variants were missing, then passed.
- `rtk cargo test -p tikeo-server alert --all-features` passed.
- `rtk cargo test -p tikeo-server alert_rule_delivery_status_redacts_channel_targets_and_reports_readiness --all-features` passed.
- `rtk cargo fmt --all -- --check` passed.
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `rtk cargo test --workspace --all-features` passed: 125 tests across workspace suites.
- `rtk cargo build --workspace --all-features` passed.
- `rtk cargo run -- --help` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm` passed.
- `rtk cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` passed.
- `rtk bash -lc 'cd web && bun run lint && bun run typecheck && bun test && bun run build'` passed.
- `rtk bash -lc 'cd sdks/java && ./gradlew test --warning-mode all --no-daemon'` passed.

### 2026-05-24 — Web route-level login bypass hardening
- `/login` now redirects authenticated client sessions at the route layer before rendering `LoginPage`; `/` continues to default to the dashboard overview.
- Regression coverage locks the route wrapper and root dashboard default.
Verification evidence:
- Targeted route test failed before implementation and passed after.
- Web lint, typecheck, targeted route test, and production build passed through RTK.


### 2026-05-24 — Phase 108 alert delivery attempt history
- Added durable `alert_delivery_attempts` storage and API foundation for alert channel delivery history/retry state, with no database foreign keys.
- Script governance firing alert delivery now records provider, redacted target, delivery status, status code/error, attempt number, retry state, and next retry time per channel.
- `GET /api/v1/alert-delivery-attempts` supports event/rule/provider/retry_state filters and is represented in OpenAPI.
- Remaining alert hardening: retry/backoff/DLQ processing, email/SMTP, and live external provider smoke.
Verification evidence:
- Targeted governance history, alert suite, OpenAPI route, storage migration, fmt, and clippy checks passed via RTK.
- Full workspace/backend, Rust SDK, Web, and Java SDK verification command passed via RTK.


### 2026-05-24 — Phase 109 dispatch latency metrics
- Added completed dispatch queue latency rollups and Prometheus snapshot recording for `tikeo_dispatch_queue_dispatch_latency_seconds`.
- Grafana template and regression coverage now include the dispatch latency query.
- Remaining observability gap: live Prometheus/Grafana recording-rule validation and real OTLP collector/export smoke.
Verification evidence:
- RED/green metrics summary test plus fmt, clippy, targeted metrics, and Grafana dashboard tests passed via RTK.


### 2026-05-24 — Phase 110 email SMTP delivery foundation
- Added local-loopback SMTP email delivery foundation for alert channels, including recipients/smtp_url/from channel fields and readiness validation.
- Email remains production fail-closed outside explicit local loopback SMTP policy; production SMTP TLS/auth/secret handling remains future work.
Verification evidence:
- RED/green local SMTP delivery test plus fmt, clippy, email delivery, and delivery-status tests passed via RTK.


### 2026-05-24 — Phase 111 alert retry/DLQ foundation
- Added due retry scan/update storage helpers, bounded retry processor, dead-letter state handling, and `POST /api/v1/alert-delivery-attempts:retry-due`.
- Retry processing appends new attempt rows while consuming old retry rows; unmatchable/exhausted attempts are marked `dead_letter`.
- Remaining alert gap: production SMTP TLS/auth/secret handling, continuous background retry scheduling, and live external provider smoke.
Verification evidence:
- RED/green retry processor test plus fmt, clippy, and alert suite passed via RTK.
- Full verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm; cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings; cd web; bun run lint; bun run typecheck; bun test; bun run build; cd ../sdks/java; ./gradlew test --warning-mode all --no-daemon'`.

### 2026-05-24 — Phase 112 alert retry background worker
- Added enabled-by-default `alert_retry` config for interval, batch size, max attempts, and retry backoff.
- Server startup now runs an ownership-gated background retry worker alongside HTTP, Worker Tunnel, schedule tick, and dispatch loops.
- Retry scans skip automatically when cluster status cannot schedule, keeping Raft followers from processing shared retry state.
- Remaining alert gap: production SMTP TLS/auth/secret handling and live external provider smoke.
Verification evidence:
- RED config test failed before `AlertRetryConfig` existed, then passed.
- RED ownership-gate test failed before `retry_once_if_owner` existed, then passed.
- Targeted fmt and clippy for `tikeo-server` / `tikeo-config` passed via RTK.
- Full verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm; cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings; cd web; bun run lint; bun run typecheck; bun test; bun run build; cd ../sdks/java; ./gradlew test --warning-mode all --no-daemon'`.

### 2026-05-24 — Phase 113 tenant scope management API foundation
- Added persistent worker-pool metadata using soft namespace/app links and no database foreign keys.
- Added scope repository operations and authenticated `/api/v1/namespaces`, `/api/v1/apps`, and `/api/v1/worker-pools` create/list routes.
- Seeded tenant read/manage RBAC permissions and added OpenAPI coverage for the new management API.
- Remaining tenant gap: full web UI, destructive lifecycle/cascade policy, and OIDC identity-to-tenant mapping.
Verification evidence:
- RED management API test failed before the routes existed, then passed after implementation.
- Targeted fmt, clippy for storage/server, migration, OpenAPI, and management API tests passed via RTK.
- Full verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm; cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings; cd web; bun run lint; bun run typecheck; bun test; bun run build; cd ../sdks/java; ./gradlew test --warning-mode all --no-daemon'`.

### 2026-05-24 — Phase 114 tenant scope management UI
- Added Web client methods for namespace, app, and Worker Pool create/list APIs.
- Added governed `/scopes` route and menu entry for `tenants:read`.
- Added `ScopesPage` with focused create cards guarded by `tenants:manage` plus metadata tables for namespace/app/Worker Pool visibility.
- Remaining tenant gap: destructive lifecycle/cascade policy and OIDC identity-to-tenant mapping.
Verification evidence:
- RED Web client/page tests failed before the API exports and page existed, then passed after implementation.
- Web lint, typecheck, targeted tests, and production build passed via RTK.
- Full verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm; cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings; cd web; bun run lint; bun run typecheck; bun test; bun run build; cd ../sdks/java; ./gradlew test --warning-mode all --no-daemon'`.

### 2026-05-24 — Phase 115 tenant scope lifecycle policy
- Added guarded DELETE routes for namespaces, apps, and Worker Pool metadata.
- Namespace deletion now rejects non-empty scopes with apps, Worker Pools, or jobs; app deletion rejects remaining Worker Pools or jobs.
- Worker Pool metadata can be deleted without affecting online Worker sessions or job records.
- Added Web console delete actions with confirmation copy that documents the non-empty rejection policy.
- Remaining tenant gap: OIDC identity-to-tenant mapping and advanced tenant isolation policy UI.
Verification evidence:
- RED backend delete lifecycle test failed with 404 before DELETE routes existed, then passed after implementation.
- RED Web page test required delete client/actions/confirm copy and passed after implementation.
- Targeted storage/server clippy, backend lifecycle/OpenAPI tests, Web lint/typecheck/targeted test/build passed via RTK.

- Phase115 full verification passed: rtk bash -lc 'cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo build --workspace --all-features && cargo run -- --help >/tmp/tikeo-help.out && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm && cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings && cd web && bun run lint && bun run typecheck && bun test && bun run build && cd ../sdks/java && ./gradlew test --warning-mode all --no-daemon'

### 2026-05-24 — Phase 116 OIDC token exchange boundary
- Added an OIDC callback token-exchange boundary that posts authorization codes to the configured provider token endpoint with client credentials.
- Callback now requires an `access_token` response but still fails closed before session issuance until external identity mapping and user mapping land.
- Split OIDC network exchange helpers into `crates/tikeo-server/src/http/oidc.rs` to keep auth routing focused.
- Remaining OIDC gap: OIDC user-info subject mapping, nonce/state persistence, user/role/tenant mapping, and opaque session issuance.
Verification evidence:
- RED/green mock IdP test covers code exchange and proves the callback does not create a session from an unverified ID token.
- Targeted OIDC tests and tikeo-server clippy passed via RTK.

- Phase116 full verification passed: rtk bash -lc 'cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo build --workspace --all-features && cargo run -- --help >/tmp/tikeo-help.out && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm && cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings && cd web && bun run lint && bun run typecheck && bun test && bun run build && cd ../sdks/java && ./gradlew test --warning-mode all --no-daemon'

### 2026-05-24 — Phase 117 OIDC UserInfo discovery boundary
- Added OIDC provider discovery and UserInfo retrieval after authorization-code token exchange.
- Callback now requires provider discovery `userinfo_endpoint` plus a non-empty key set, but still fails closed before trusting `access_token` signatures or creating sessions.
- Extended the mock IdP regression test to prove token, discovery, and UserInfo endpoints are each reached once while preserving the `{ code, message, data }` failure envelope.
- Remaining OIDC gap: OIDC user-info subject mapping, role/tenant mapping, nonce/state hardening, and opaque session issuance.
Verification evidence:
- Targeted OIDC tests and tikeo-server clippy passed via RTK.

- Phase117 full verification passed: rtk bash -lc 'cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo build --workspace --all-features && cargo run -- --help >/tmp/tikeo-help.out && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm && cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings && cd web && bun run lint && bun run typecheck && bun test && bun run build && cd ../sdks/java && ./gradlew test --warning-mode all --no-daemon'

### 2026-05-24 — Phase 118 OIDC state/UserInfo opaque-session correction
- Corrected the OIDC direction: tikeo login state remains opaque session tokens in `auth_sessions` plus moka cache; provider tokens are never used as local session state.
- Added persisted hashed OIDC authorization states with one-time callback consumption and replay rejection.
- Replaced the current provider-token-as-session path with token exchange + provider UserInfo fetch, then fail-closed until external subject mapping creates a local opaque tikeo session.
- Added `oidc_auth_states` storage/entity/repository support with soft, standalone metadata and no foreign keys.
- Remaining OIDC gap: external subject to local user/role/tenant mapping, nonce/state hardening, and opaque session issuance from mapped identity.
Verification evidence:
- Targeted OIDC tests cover generated state, one-time state consumption, token exchange, UserInfo fetch, and fail-closed local session mapping.
- Storage and server clippy passed via RTK after the correction.

- Phase118 full verification passed: rtk bash -lc 'cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo build --workspace --all-features && cargo run -- --help >/tmp/tikeo-help.out && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm && cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings && cd web && bun run lint && bun run typecheck && bun test && bun run build && cd ../sdks/java && ./gradlew test --warning-mode all --no-daemon'


### 2026-05-24 — Phase 119 real OTLP exporter smoke
- Completed the OpenTelemetry distributed tracing Phase 3 item with real OTLP HTTP exporter startup wiring.
- Added focused `observability::tracing::TracingRuntime` using OpenTelemetry SDK plus `tracing-opentelemetry`, keeping local tracing disabled-by-default unless configured.
- Added local collector smoke coverage proving exported spans POST a non-empty OTLP protobuf payload to `/v1/traces` and carry configured exporter headers.
- Remaining observability gap is Prometheus/Grafana recording-rule validation, not OTLP tracing.
Verification evidence:
- `rtk cargo test -p tikeo-server --test otel_exporter_smoke --all-features` passed.
- `rtk cargo test -p tikeo-server observability_status_reports_default_and_configured_otlp_without_collector --all-features` passed.
- Targeted `rtk cargo fmt --all -- --check` and `rtk cargo clippy -p tikeo-server --all-targets --all-features -- -D warnings` passed.
- Phase119 full verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out'`.
- SDK/Web verification passed: `rtk bash -lc 'set -euo pipefail; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm; cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings; cd web; bun run lint; bun run typecheck; bun test; bun run build; cd ../sdks/java; ./gradlew test --warning-mode all --no-daemon'`.


### 2026-05-24 — Phase 120 Java Spring Boot Starter lifecycle completion
- Completed the Java Spring Boot Starter SDK runtime behavior with a `TikeoWorkerLifecycle` SmartLifecycle bridge.
- Starter now auto-starts and stops the configured `TikeoWorkerClient` with the Spring application lifecycle while preserving processor scanning.
- Added `tikeo.worker.enabled` and `tikeo.worker.auto-startup` controls for disabling worker beans or manual startup.
- Updated the Spring worker demo so lifecycle ownership lives in the starter instead of the demo runner.
Verification evidence:
- RED starter test failed before `TikeoWorkerLifecycle` existed, then passed after implementation.
- `rtk bash -lc 'cd sdks/java && ./gradlew :tikeo-spring-boot-starter:test --warning-mode all --no-daemon'` passed.
- Phase120 Java verification passed: `rtk bash -lc 'set -euo pipefail; cd sdks/java; ./gradlew test --warning-mode all --no-daemon; cd ../../examples/java/spring-worker-demo; ./gradlew build --warning-mode all --no-daemon'`.

### 2026-05-24 — Worker identity lifecycle design
- Added `design/worker-identity-lifecycle-design.md` to separate the production design for Worker Pool / Logical Worker / Worker Session identity, status reason evidence, generation fencing, lease expiry, replacement, graceful shutdown, history retention, UI layering, and SDK defaults.
- Updated the architecture design with the `worker_id` session/incarnation semantics and added Worker identity/session lifecycle governance to the Phase 4 roadmap and innovation capability table.
Verification evidence:
- Documentation-only change reviewed with `rtk git diff --check`.


### 2026-05-24 — Worker identity lifecycle design: bare metal support
- Extended `design/worker-identity-lifecycle-design.md` so Worker identity/session lifecycle governance explicitly supports bare metal, VM, systemd, Supervisor, and Windows Service deployments in addition to K8s/Docker.
- Added host-id + instance-slot identity guidance, auto identity-mode precedence, and route-map updates in the architecture roadmap wording.
Verification evidence:
- Documentation-only change reviewed with `rtk git diff --check`.

### 2026-05-24 — Phase3/Phase4 service-priority rebalance
- Reorganized remaining Phase 3 / Phase 4 roadmap around service usability instead of chronological feature accumulation.
- Raised P0 priority for OIDC opaque-session mapping, real TLS/mTLS listeners, Worker identity/session lifecycle governance, deployment/operations bootstrap, and production alert delivery hardening.
- Moved migration tools, Terraform/GitOps/CRD, dependency topology, intelligent scheduling, plugin system, webhook/event sources, and canary/version rollback into P2 because they do not block initial shared-service adoption.
Verification evidence:
- Documentation-only change reviewed with `rtk git diff --check`.

### 2026-05-25 — P0 OIDC mapped opaque session issuance
- Completed P0 OIDC external-subject mapping: `oidc_identities` maps `(issuer, subject)` to a local username plus optional namespace/app/worker_pool bindings without database foreign keys.
- OIDC callback now uses provider tokens only for token exchange/UserInfo, then issues a local opaque `atk_` session from `auth_sessions` + moka after mapping succeeds; unmapped identities still fail closed.
- Split new code by responsibility: storage identity repository, OIDC callback completion, and session metadata encoding are separate modules; no new clippy allow was added.
Verification evidence:
- RED mapped-subject callback test failed before `OidcIdentityRepository` existed, then passed.
- Targeted OIDC tests, storage/server clippy, and fmt check passed via RTK.
- P0 OIDC mapped opaque session full backend verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out'`.

### 2026-05-25 — P0 real TLS/mTLS listeners
- Added real HTTP HTTPS serving with rustls and a TLS smoke test that reaches an axum route over `https://127.0.0.1`.
- Added shared TLS material loading for rustls/tonic, Worker Tunnel TLS/mTLS startup wiring, and a `WorkerTunnelRuntime` dependency bundle to avoid adding clippy argument-count allowances.
- HTTP TLS rebuilds the acceptor from configured files for each new connection so certificate/key/CA file rotation is picked up without process restart.
- Transport security diagnostics now report `plaintext`, `tls`, `mtls`, or `tls_config_error` and check certificate/key/CA file readability instead of claiming `tls_pending_listener`.
Verification evidence:
- `rtk cargo test -p tikeo-server http_tls_listener_serves_https_when_configured --all-features` passed.
- `rtk cargo test -p tikeo-server transport_security_status_reports_defaults_and_partial_mtls_config --all-features` passed.
- Full backend verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out'`.

### 2026-05-25 — P0 Worker lifecycle Slice A: generation/fencing baseline
- Reviewed `design/worker-identity-lifecycle-design.md` and implemented the first Worker identity lifecycle slice against that design.
- `WorkerRegistered` now returns `generation` and `fencing_token`; Heartbeat carries both and stale/replaced session heartbeats are rejected.
- `WorkerRegistry` now derives a logical instance key from namespace/app/cluster/region/client_instance_id, increments generation for repeat registration, marks old sessions `replaced` with `replaced_by_new_generation`, and filters dispatch/worker list to latest schedulable sessions.
- Rust and Java SDK heartbeats now echo the assigned generation/fencing token.
Verification evidence:
- RED registry replacement test failed before implementation, then passed.
- `rtk cargo test -p tikeo-server worker --all-features` passed.
- `rtk bash -lc 'cd sdks/java && ./gradlew test --warning-mode all --no-daemon'` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm` and Rust SDK clippy passed.
- Full backend verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out'`.


### 2026-05-25 — P0 Worker lifecycle Slice B: persistent lifecycle store
- Reviewed `design/worker-identity-lifecycle-design.md` again before implementation and continued the approved Worker identity/session lifecycle design.
- Added persistent `worker_logical_instances`, `worker_sessions`, and `worker_session_events` entities/migration/SQLite compatibility initialization with soft links only, no database foreign keys.
- Added `WorkerLifecycleRepository` for logical key upsert, generation increment, replacement marking, event recording, and fenced heartbeat lease renewal.
- Wired production server startup to create `WorkerRegistry::with_lifecycle(...)`, so Worker Tunnel registration/replacement/heartbeat now persists lifecycle state while tests can still use in-memory registries.
- Avoided `#[allow(clippy::too_many_lines)]`; split lifecycle storage into dedicated entity/repository modules and refactored migration down helpers to keep clippy clean.
Verification evidence:
- RED storage lifecycle test failed before repository types existed, then passed after implementation.
- `rtk cargo clippy -p tikeo-storage --all-targets --all-features -- -D warnings` passed.
- `rtk cargo clippy -p tikeo-server --all-targets --all-features -- -D warnings` passed.
- `rtk cargo test -p tikeo-storage --all-features` passed.
- `rtk cargo test -p tikeo-server worker --all-features` passed.
- `rtk cargo build -p tikeo-server --all-features` passed.
- Serve smoke with temporary SQLite config reached listener startup (`timeout` 124 => `smoke_started`) without missing-table startup errors.


### 2026-05-25 — P0 Worker lifecycle Slice C: lease scanner
- Added persistent lease scanner support for Worker lifecycle governance.
- `WorkerLifecycleRepository::mark_expired_online_sessions` scans expired `online` sessions and marks them `offline` with reason `lease_expired_unknown`, writes a `lease_expired` event, and degrades the current logical worker only when the expired session is still current.
- Added `tunnel::lifecycle::run_lease_scanner` and wired it into server startup as a background maintenance task.
- Preserved design constraint: heartbeat timeout evidence says lease expired without graceful/replacement/transport evidence and does not call it a crash.
- Kept module split; no new clippy allow was added.
Verification evidence:
- RED storage test failed before `mark_expired_online_sessions` existed, then passed.
- `rtk cargo clippy -p tikeo-server --all-targets --all-features -- -D warnings` passed after extracting `run_worker_lease_scanner` instead of adding a too-many-lines allow.
- `rtk cargo test -p tikeo-storage worker_lifecycle --all-features` passed.


### 2026-05-25 — P0 Worker lifecycle Slice D: graceful unregister across Rust and Java SDKs
- Added Worker Tunnel `UnregisterWorker` protocol message to server, root proto, Rust SDK proto, and Java SDK proto.
- Added persistent `WorkerLifecycleRepository::graceful_unregister` to mark current fenced sessions as `stopped / graceful_shutdown` and record a `graceful_shutdown` event.
- Added `WorkerRegistry::unregister` and server tunnel handling for active worker-initiated unregister messages.
- Rust `WorkerSession::close()` now sends unregister with worker_id/generation/fencing_token.
- Java `GrpcTikeoWorkerClient.close()` now sends unregister with worker_id/generation/fencing_token before completing the stream.
- Kept Java and Rust SDK behavior aligned per user direction.
Verification evidence:
- RED storage and Rust SDK close tests failed before implementation, then passed.
- `rtk cargo clippy -p tikeo-server --all-targets --all-features -- -D warnings` passed after splitting the oversized Worker message handler instead of adding allow.
- `rtk cargo test -p tikeo-storage worker_lifecycle --all-features` passed.
- `rtk cargo test -p tikeo-server worker --all-features` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml worker_session_close_sends_graceful_unregister --features wasm` passed.
- `rtk cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` passed.
- `rtk bash -lc 'cd sdks/java && ./gradlew :tikeo:test --tests net.tikeo.core.GrpcTikeoWorkerClientTest --warning-mode all --no-daemon'` passed.


### 2026-05-25 — P0 Worker lifecycle Slice E: assignment token validation
- Added `assignment_token` to `DispatchTask`, `TaskResult`, and `TaskLog` across root proto, server proto, Rust SDK proto, and Java SDK proto.
- WorkerRegistry now generates per-dispatch assignment tokens, stores active tokens on current worker sessions, and rejects missing/wrong tokens for task logs/results.
- Rust SDK echoes dispatch assignment token on task results and keeps logs token-empty unless emitted from a future task context.
- Java SDK echoes dispatch assignment token on task results.
- Added server tests for token generation/validation and wrong-token result rejection.
Verification evidence:
- RED SDK tests failed before token echo; RED server test failed before `accepts_worker_assignment`; then all passed after implementation.
- `rtk cargo clippy -p tikeo-server --all-targets --all-features -- -D warnings` passed.
- `rtk cargo test -p tikeo-server worker --all-features` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm` passed.
- `rtk cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` passed.
- `rtk bash -lc 'cd sdks/java && ./gradlew :tikeo:test --tests net.tikeo.core.GrpcTikeoWorkerClientTest --warning-mode all --no-daemon'` passed.


### 2026-05-25 — P0 Worker lifecycle Slice F: Web layered history UI
- Added HTTP `/api/v1/workers/history` backed by persistent worker lifecycle sessions/events.
- Added DTOs and storage list methods for Worker session history and lifecycle events.
- Worker cluster page now fetches online workers, dispatch queue, and lifecycle history together.
- Added `WorkerLifecycleHistory` panel with 在线 / 异常/待确认 / 历史 segmentation and recent lifecycle event timeline.
Verification evidence:
- `rtk cargo clippy -p tikeo-storage --all-targets --all-features -- -D warnings` passed.
- `rtk cargo clippy -p tikeo-server --all-targets --all-features -- -D warnings` passed.
- `rtk cargo test -p tikeo-storage worker_lifecycle --all-features` passed.
- `rtk cargo test -p tikeo-server worker --all-features` passed.
- `rtk bash -lc 'cd web && bun run lint && bun run typecheck && bun test src/pages/__tests__/WorkersPage.test.tsx && bun run build'` passed.

## 2026-05-25 P0 deployment/operations bootstrap

- P0 部署与运维 bootstrap 已补齐 Compose/systemd/裸机模板：`deploy/compose/tikeo.env.example`、`deploy/systemd/tikeo.service`、`deploy/systemd/tikeo.env`、`deploy/bare-metal/check-config.sh`。
- 根 `docker-compose.yml` 支持镜像、端口和数据卷通过环境变量覆盖，便于本地生产化试跑。
- 新增 `scripts/verify-deploy-bootstrap.sh` 静态校验；Helm 继续后置到外部 DB、secret、网关和 TLS 生产参数稳定后。

## 2026-05-25 P0 alert delivery hardening

- P0 生产告警投递硬化已补齐：Email channel 支持 `smtps://` / `smtp+starttls://`、AUTH LOGIN、`smtp_url_secret_ref` 与 `password_secret_ref` 环境变量引用；明文 `smtp://` 仅保留 loopback smoke 场景。
- 告警投递状态 API 增强为返回脱敏 target 与 transport security；新增 `GET /api/v1/alert-delivery-attempts:queue-status` 汇总 delivered/retry_pending/dead_letter/retry_consumed 并返回最近 DLQ。
- Web 新增“告警投递”运维页面，展示 retry/DLQ 汇总与最近 DLQ 明细；Compose/systemd env 模板补充 `TIKEO_ALERT_SECRET_` secret ref 约定。
- 验证：`cargo test -p tikeo-server alert::`、`cargo test -p tikeo-server http::tests::alert`、`cargo test -p tikeo-config --lib`、`cargo clippy -p tikeo-server --all-targets --all-features -- -D warnings`、`cargo clippy -p tikeo-config --all-targets --all-features -- -D warnings`、`cd web && bun run typecheck && bun run lint && bun test src/pages/__tests__/AlertDeliveryPage.test.tsx`。

## 2026-05-25 P0 completion checkpoint

- P0 服务使用 / 生产上线阻塞项全部完成：OIDC opaque session、HTTP/Worker Tunnel TLS/mTLS、Worker 身份与会话生命周期治理、部署运维 bootstrap、生产告警投递硬化均已落地并提交。
- 按用户要求 P0 完成后停止，不自动进入 P1/P2。

### 2026-05-25 — Module/file-size cleanup and script release-gate preview
- Enforced the new single-source-file size rule by splitting oversized Rust files: `http/mod.rs` is now a module entry, HTTP state/router/server/health/tests live in focused modules, raft-rs tests moved out, migration identifiers/index/column helpers split out, and workflow repository types/conversions/validation/queue/events split by responsibility.
- Added `GET /api/v1/scripts/{id}/release-gate` as a local, read-only script production-gate preview: it reports whether a version is currently releasable, blocking reasons, required operator actions, and that signature verification is not enabled yet.
- Verified max source file line count is 1495 across `crates`, `src`, `sdks`, `web`, and `examples` for Rust/TS/TSX/Java/JS sources.
Verification evidence:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.
- Targeted release-gate tests passed: `cargo test -p tikeo-server script_release_gate_preview --all-features`; `cargo test -p tikeo-server script_publish_blocks_legacy_dangerous_policy_snapshot --all-features`.

### 2026-05-25 — P1 script signature local verification boundary
- Added `script_governance.release_signature_secret_ref` configuration, disabled by default so approval/signature metadata remains fail-closed unless an operator explicitly configures verification.
- Script publish/rollback now verifies `approval_ticket` + `signature` when configured with an `env:` secret ref; the signature binds script id, immutable version number, content SHA-256, and approval ticket.
- Release-gate preview now reports whether signature verification is configured.
- Preserved the source-size rule; max source file line count remains 1495.
Verification evidence:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.

### 2026-05-25 — P1 script release signature metadata persistence
- Persisted verified script release signature metadata on the release pointer: approval ticket, signature digest, verification timestamp, and verifier identity.
- Exposed the metadata through `ScriptSummary` so script list/detail/publish/rollback responses can show signed release evidence.
- Web Scripts page now shows release signature status in the table and signature details in the drawer.
- Existing SQLite dev databases gain compatibility columns automatically; unsigned safe releases remain supported, while signed metadata is only stored after configured verification succeeds.
- Preserved the source-size rule; max checked source file line count remains 1495.
Verification evidence:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.
- `cd web && bun run typecheck && bun run lint && bun test && bun run build` passed.

### 2026-05-25 — P1 script release grant payload boundary
- Added `ScriptReleaseGrantSet` in core and `ScriptReleaseRequest.grants` in HTTP/OpenAPI for URL/File/Secret grant payloads.
- Grant payload categories are explicit (`url`, `file_read`, `file_write`, `secret`) and validate malformed empty/untrimmed entries.
- Any non-empty grant remains fail-closed with a business error until verified release grant enforcement is implemented; no Worker-side access is enabled.
- Web API client types now include the same release grant request shape.
- Preserved the source-size rule; max checked source file line count remains 1495.
Verification evidence:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.
- `cd web && bun run typecheck && bun run lint && bun test && bun run build` passed.

### 2026-05-25 — P1 script release grant evidence persistence
- Added release-pointer storage for verified URL/File/Secret grant evidence (`release_grants_json`, `release_grants_verified_at`, `release_grants_verified_by`) with SQLite compatibility migration.
- `ScriptSummary` now exposes optional `release_grants` evidence; Web Scripts detail can display verifier, verification time, URL grants, and Secret grants when present.
- Repository tests now cover persisting and reloading verified grant evidence.
- HTTP publish/rollback still pass no verified grant evidence and non-empty `grants` remain fail-closed; no Worker-side URL/File/Secret access is enabled.
- Preserved the source-size rule; max checked source file line count remains 1495.
Verification evidence:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.
- `cd web && bun run typecheck && bun run lint && bun test && bun run build` passed.

### 2026-05-25 — P1 local signed release grants
- Completed the local script release governance loop for URL/File/Secret grants: configured env-secret signatures now bind canonical grants JSON in addition to script id, immutable version number, content SHA-256, and approval ticket.
- Publish/rollback persist `release_grants` evidence only after signature verification succeeds; unconfigured systems still reject non-empty grants and approval/signature metadata.
- The previous fail-closed behavior remains for deployments without `script_governance.release_signature_secret_ref`.
- Worker-side URL/File/Secret access remains disabled; this slice only completes release governance verification/evidence.
- Roadmap now marks the P1 script approval/signature/grants production gate subitem complete and shifts next P1 priority to OIDC tenant/app/role binding and advanced tenant isolation UI.
- Preserved the source-size rule; max checked source file line count remains 1495.
Verification evidence:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.
- `cd web && bun run typecheck && bun run lint && bun test && bun run build` passed.

## 2026-05-25 — P1 Worker runtime grant enforcement
- Closed the signed grant handoff from release pointer to Worker dispatch binding: Server now includes `allowed_network_hosts`, read-only paths, writable paths, secret refs, and `allow_network` derived from verified `release_grants`.
- Rust SDK now carries `allowed_network_hosts` through `ScriptRunnerPolicy`; local subprocess remains fail-closed for grants; container runner mounts signed file grants and rejects network/secret grants without safe runtime enforcement.
- Java SDK proto/test updated so grant-bearing script bindings are understood and still reported unsupported without invoking user processors.

## 2026-05-25 — P1 OIDC tenant scope mapping
- Added governed OIDC identity mapping APIs (`/api/v1/oidc-identities`) for issuer+subject -> local user plus namespace/app/worker-pool scope bindings.
- OIDC session responses now include scope metadata, matching `/auth/me`, so UI can show tenant-limited sessions immediately after callback.
- Scopes page now manages OIDC mappings with fail-closed copy: unmapped external subjects cannot obtain local tikeo sessions.

## 2026-05-25 — P1 Prometheus/Grafana recording-rule validation
- Added Prometheus recording rules and scrape config under `observability/prometheus/`.
- Added optional Compose observability profile for local Prometheus scrape/rule smoke.
- Updated Grafana dashboard SLO panels to query stable `tikeo:*` recording series and added regression coverage for dashboard/rule coherence.

## 2026-05-25 — P1 Go SDK dry-run foundation
- Added independent Go SDK module under `sdks/go/tikeo` with config validation, registration/heartbeat message shapes, processor/outcome interfaces, official Go gRPC/protobuf dependency boundary, vendored Worker Tunnel proto, and dry-run client tests.
- Added standalone Go worker demo under `examples/go/worker-demo`; no server Dockerfile coupling and no generated protobuf dependency yet.

### 2026-05-25 P1 Go SDK official gRPC/protobuf foundation
- Added Go SDK foundation under `sdks/go/tikeo` with official `google.golang.org/grpc` ClientConn creation, official protobuf/grpc generated Worker Tunnel bindings, endpoint normalization, dry-run registration/heartbeat/task interfaces, and standalone demo tests.
- User explicitly deferred Python and Node.js SDKs; next SDK slice should stay on Go Worker Tunnel run-loop ergonomics.

### 2026-05-25 Phase4 P0 Worker lifecycle transport evidence
- Continued Worker identity/session lifecycle governance strictly against `design/worker-identity-lifecycle-design.md`. Added transport-error evidence path so gRPC stream errors or non-graceful stream end mark the current session offline/degraded with `transport_error` instead of waiting for lease timeout.
- Confirmed Python/Node SDK and remaining Go SDK run-loop work are deferred; next Phase4 P0 slice is deployment/operations bootstrap.

### 2026-05-25 Phase4 P0 deployment bootstrap
- Completed Compose/systemd/bare-metal deployment bootstrap docs and templates, including stable Worker identity env guidance, systemd Rust worker demo unit/env, and `deploy/smoke/worker-bootstrap-smoke.sh` readiness + dry-run worker check.
- Go run-loop/Python/Node SDK work remains deferred; Helm stays deferred until external DB, secrets, gateway, and TLS parameters stabilize.

- 2026-05-27 09:38: Locked script language identifiers to full JavaScript / TypeScript values so the web script editor can select explicit CodeMirror JavaScript vs TypeScript linting modes. Legacy js / ts aliases remain parse-compatible; dispatch and Java worker capabilities now canonicalize to script:javascript / script:typescript. Verified with targeted Rust, Web, and Java tests.

- 2026-05-27 09:51: Removed raw WASM from Web script create/edit language options. Direct language=wasm remains documented as a historical/low-level compatibility path, while normal scripts use sandbox.backend auto/wasmtime/wasmedge/srt/deno/v8/docker/podman/custom instead of WASM as a script type.

- 2026-05-27 09:54: Added local dev seed script examples and API jobs for every Web script language enum: shell, python, javascript, typescript, powershell, and rhai. Applied scripts/dev-seed.sh to tikeo-dev.db and verified six script_language_examples plus six script_jobs.

- 2026-05-27 12:55: Changed script dispatch matching to unified worker capability `script` so Python/JavaScript/TypeScript/etc. are dispatched to script-capable workers instead of being blocked by missing `script:<language>` capability. Legacy `script:<language>`, `script:*`, and `*` remain compatible for normal scripts; direct WASM modules still require `script:wasm`. Worker-side sandbox selection remains based on binding language plus sandbox.backend.

- 2026-05-27 14:10: Fixed Java demo shell script execution path: sandbox=auto now resolves native scripts to srt/native-script semantics, Spring starter registers a development-only local shell subprocess runner instead of sending real shell scripts through the limited bundled WASI shell micro-runtime, and the demo image config now uses JavaScript/TypeScript keys. Verification: Java SDK script tests, Spring starter auto-configuration tests, and spring-worker-demo tests passed.

### 2026-05-31 — SDK API-Key Service Account lifecycle closed
- Service Account is now a first-class app-scoped machine identity with HTTP management routes, storage entity/repository, SQLite migration compatibility, Web management table, and API client helpers.
- API-Key creation no longer accepts an implicit service account name; it requires `service_account_id` for an existing active identity. Disabled identities revoke active keys and fail SDK-key authentication.

### 2026-06-02 — 联合自动化测试状态复核完成
- `design/server-web-java-joint-executable-test-status-plan.md` 当前总览已全绿：P0-A 17/17、P0-B 16/16、P0-C 12/12、P0-D 10/10、P1-E 7/7、P1-F 9/9、P2-G 6/6、数据库专项 3/3。
- `design/server-web-java-joint-automation-test-plan.md` 已同步清理旧的待执行状态，所有测试项、环境/端口、CI 分层和排障资产均按现有证据更新为通过/已配置/已沉淀。
- Server + Web + Java SDK/Demo 自动化联调测试当前为可验收状态；真实浏览器 screenshot/video CI 产物属于后续增强，不再作为当前测试闭环阻塞项。

### 2026-06-02 — Java SDK Spring Boot starter compatibility corrected
- Main `tikeo-spring-boot-starter` remains the Spring Boot 4.x starter.
- Added compatibility modules: `tikeo-spring5`, `tikeo-spring6`, `tikeo-spring-boot2-starter`, and `tikeo-spring-boot3-starter`.
- All Java SDK modules compile with `--release 17`; Spring demo validates Boot 3.x through `tikeo-spring-boot3-starter`.

### 2026-06-02 — Java compat modules have real source boundaries
- Boot 2/3 compatibility modules now contain explicit `src/main` and `src/test` directories instead of relying on hidden Gradle source-set indirection.
- Full Java SDK and Spring demo tests pass with the explicit module layout.

### 2026-06-02 — Java demo 补齐 Spring Boot starter 兼容用例
- Java Spring Worker Demo 新增 `SpringBootStarterCompatibilityMatrixTest`，明确覆盖 Boot3 demo 使用 `tikeo-spring-boot3-starter` 的用例。
- 用例同时检查 SDK 层 Boot2/Boot3/Boot4 starter 与 Spring5/Spring6 adapter 都是带真实源码/测试/资源元数据的模块，避免空模块或 Gradle sourceSet 伪兼容。
- Demo README 已补充 starter 兼容矩阵与测试项说明。
- 验证：`cd examples/java/spring-worker-demo && ./gradlew clean test --no-daemon`；`cd sdks/java && ./gradlew clean test --no-daemon`；`git diff --check -- examples/java/spring-worker-demo sdks/java .memory`。

### 2026-06-02 — Java demo 按 Spring Boot 2/3/4 独立拆分
- `examples/java` 下新增三个独立 demo：`spring-boot2-worker-demo`、`spring-boot3-worker-demo`、`spring-boot4-worker-demo`。
- 每个 demo 独立 Gradle 工程/工作目录，分别依赖 `tikeo-spring-boot2-starter`、`tikeo-spring-boot3-starter`、`tikeo-spring-boot-starter`，并保留 processor、worker lifecycle、management API、script/API/plugin 用例测试。
- Boot2 demo 使用 Spring Boot 2.7 BOM 方式规避 Boot2 Gradle plugin 与当前 Gradle 9.5.1 API 冲突，但仍是标准 Spring Boot 2 应用与 `@SpringBootTest` 用例。
- 验证：三套 demo 均在各自目录执行 `./gradlew clean test --no-daemon` 通过。

### 2026-06-02 — 删除旧 `spring-worker-demo` 泛化 demo
- 删除 `examples/java/spring-worker-demo`，Java demo 入口统一为 `spring-boot2-worker-demo`、`spring-boot3-worker-demo`、`spring-boot4-worker-demo`。
- 当前联调脚本默认使用 Boot3 demo；Boot2/Boot4 demo 作为版本兼容验证独立保留。

### 2026-06-02 — Java SDK 改为根聚合 + 子模块独立 Gradle
- `sdks/java/build.gradle.kts` 只保留聚合与 group/version；各 SDK 子模块新增自己的 `build.gradle.kts`。
- 每个子模块独立声明插件、依赖、测试、`maven-publish`，便于独立约束 Spring/Boot/grpc/protobuf 版本与独立发布。
- 验证：`cd sdks/java && ./gradlew clean test publishToMavenLocal --no-daemon`；Boot2/Boot3/Boot4 demo 测试均通过。

### 2026-06-05 — 文档同步：2026-06-04 Worker/SDK parity 与持久化状态
- 架构文档已更新 Go SDK 当前状态、Java Boot2/3/4 starter 兼容模块、Worker Tunnel `OpenTunnel` proto 名称、Go/Rust SDK demo parity、Worker session snapshot 持久化和 Web Worker 页面分组/调度队列拆分。
- Worker identity lifecycle 文档已补充 `worker_sessions` snapshot JSON 字段、`transport_error` 事件、重启后 worker 可见性快照 Slice F 与验证计划。
- Java demo 多 Worker 联调报告已追加 2026-06-04 跨语言 Worker parity / 持久化可见性补充结果。
- 联合自动化测试状态表已追加 H-WORKER/H-GO/H-RUST/H-SCRIPT/H-CI 补充项，当前补充项 6/6 通过；测试方案新增 X-LANG 自动化计划。
- `.prompt/147-phase4-cross-language-worker-parity-and-persistence-hardening.md` 已创建，要求下一阶段把手动验收转成可重复 harness。

### 2026-06-05 — 反伪实现审计完成，跨语言 Worker parity 自动化落地
- Server 自动调度 cursor 已持久化到 `schedule_cursors`，避免重启后依赖内存 cursor 或重复触发；unknown Raft business command 现在明确 `rejected`。
- Java/Go/Rust script runner capability 改为真实广告：Unavailable/Unsupported runner 只保留 fail-closed 执行边界，不进入 worker structured scriptRunners；容器 runner 广告 canonical backend。
- Rust SDK 支持 `TaskOutcome::Success(String)`，Rust demo 与 Go/Java 一样上报可见成功消息。
- Web i18n 新增质量门，当前英文/中文 label 不再依赖机械坏翻译覆盖。
- `deploy/smoke/cross-language-worker-parity-smoke.sh` 已通过，证据目录 `.dev/reports/cross-language-workers-20260605T032108Z-202626/`。

### 2026-06-05 — Main CI now covers Go, demos, deploy tooling, and cross-language smoke
- `.github/workflows/ci.yml` now has dedicated jobs for Go SDK/demo, Terraform provider/K8s operator Go modules, Java Boot2/3/4 demos, Rust demo, and cross-language Worker parity smoke.
- Docker build validation depends on all language/demo/smoke quality gates, so fake SDK/demo regressions cannot bypass CI into image validation.


### 2026-06-05 — Storage migration versioning hardening
- Moved SQLite legacy/dev schema compatibility out of the untracked `connect_and_migrate` post-hook and into the explicit SeaORM migration `crates/tikeo-storage/src/migration/sqlite_compat.rs`.
- `connect_and_migrate` now relies on `migration::Migrator::up` only; schema compatibility upgrades are persisted in `seaql_migrations` as `sqlite_compat`.
- Split SQLite foreign-key soft-link rebuild helpers into `migration/sqlite_compat/foreign_keys.rs`, keeping touched source files under the 1500-line rule.
- Added regression coverage proving `sqlite_compat` is recorded and old SQLite dev DB shapes still get scope tables before indexes.
Verification evidence:
- `cargo test -p tikeo-storage sqlite_schema_compatibility_upgrade_is_tracked_as_versioned_migration -- --nocapture` passed.
- `cargo test -p tikeo-storage sqlite_compatibility_creates_scope_tables_before_indexes_for_existing_dev_db -- --nocapture` passed.
- `cargo test -p tikeo-storage -- --nocapture` passed.
- `scripts/db-compat-smoke.sh` passed with SQLite + Docker PostgreSQL/MySQL.

- CI policy guard remains clean: `python3 .github/tests/workflow_contract_test.py` passed; `scripts/verify-github-actions-node-runtime.py --min-node-major 24` reported 13 external actions with no runtime below node24.
- Source-size audit for touched files passed, but whole-repo audit found pre-existing over-1500-line debt in dispatcher/repository/workflow/Web generated-or-aggregate files; recorded in `.memory/next.md` instead of pretending the repo-wide rule is fully satisfied.


### 2026-06-05 — CI grouping aligned by runtime surface
- Reorganized main CI job groups to match product/runtime ownership: `Server`, `Web`, `Java SDK + demo`, `Rust SDK + demo`, `Go SDK + demo`, `Python SDK + demo / deferred`, `Node.js SDK + demo / deferred`, and `Other / ...` for workflow policy, deploy tooling, cross-language smoke, and Docker validation.
- Merged Java SDK and Boot2/Boot3/Boot4 demo checks into one Java group; merged Rust SDK and Rust demo checks into one Rust group; kept Go SDK/demo together.
- Added fail-closed deferred gates for Python and Node.js SDK/demo: CI passes while directories are absent, but fails as soon as those directories appear without real test/demo smoke wiring.
- Updated workflow contract tests to prevent regression to fragmented Java/Rust jobs or ambiguous Docker/cross-language naming.
Verification evidence:
- `python3 .github/tests/workflow_contract_test.py` passed.
- YAML parse for all `.github/workflows/*.yml` passed.
- `scripts/verify-github-actions-node-runtime.py --min-node-major 24` passed with 13 external actions and no runtime below node24.
- `git diff --check` passed.
- Per user instruction, GitHub CI result monitoring/debugging is intentionally skipped for this final grouping commit.

### 2026-06-08 — README motion polish and full Codecov surface
- README and Chinese README now use `assets/docs/tikeo-logo-breathe.gif`, a 220x220 GIF derived from the Web breathing/task-flow logo animation, instead of the static SVG logo.
- Coverage workflow now uploads reports for Rust workspace, Web, Java SDK, Go SDK, Python SDK/demo, and Node.js SDK through direct `codecov-cli` uploads with per-surface flags.
- Java SDK aggregate Gradle build now applies JaCoCo to Java library subprojects and emits XML reports for Codecov.
- Local report generation passed for Web (117 tests), Node.js SDK (14 tests), Go SDK, Java SDK (7 JaCoCo XML reports), and Python SDK/demo (19 tests).

### 2026-06-08 — Full Coverage remote green
- GitHub Actions Coverage run `27125171618` succeeded on commit `5beb036380c8fbb54f54a0ed60a01b6c366b286d`; all six coverage jobs passed and uploaded through Codecov CLI.
- Overall Codecov branch badge returned `79%`, confirming the README badge is no longer pending or Rust-only.

### 2026-06-08 — Main CI green after coverage/logo polish
- Main CI run `27125171526` succeeded for commit `5beb036380c8fbb54f54a0ed60a01b6c366b286d`, including Server/Web/SDK/demo groups, cross-language worker smoke, and Docker build validation.

### 2026-06-08 — Helm production deployment hardening
- Helm chart production baseline now covers external PostgreSQL/MySQL/CockroachDB database URL injection via Kubernetes Secret, conditional SQLite PVC persistence, service account creation, tunable resources/probes/security contexts, server/web ingress, HTTP listener TLS Secret mounts, Worker Tunnel TLS/mTLS Secret mounts, and generated `transport_security` config.
- Added examples: `values-sqlite-dev.yaml`, `values-external-postgres.yaml`, `values-ingress-tls.yaml`, and `values-worker-identity.yaml`.
- Updated Helm README with external database Secret workflow, TLS/mTLS Secret boundaries, worker identity outbound-only guidance, and rollback runbook.
- Updated deploy bootstrap verification to assert Helm production artifacts instead of the old deferred-Helm placeholder.
Verification evidence:
- RED/green contract: `python3 -m unittest deploy.tests.iac_artifacts_test.IacArtifactsTest.test_helm_chart_exposes_production_hardening_contracts` failed before implementation and passed after chart hardening.
- `python3 -m unittest deploy.tests.iac_artifacts_test deploy.tests.smoke_assertions_test` passed.
- `scripts/verify-deploy-bootstrap.sh` passed.
- `.dev/tools/helm lint deploy/helm/tikeo` passed with only the optional icon recommendation.
- `.dev/tools/helm template` passed for default, external database, and external database + TLS/mTLS values.
- Remote verification: CI run `27128044956` and Coverage run `27128044845` both completed successfully for source commit `c90b44177a692946ad4cd000f16e6653ddc508e9`.

### 2026-06-08 — Helm operations maturity overlay
- Extended the Helm chart beyond the first production baseline with optional PodDisruptionBudget, NetworkPolicy, ServiceMonitor, Gateway API `GRPCRoute`, and `values.schema.json` support.
- Added `values-ops-hardening.yaml` and `values-gateway-api-worker-tunnel.yaml` examples.
- NetworkPolicy templates preserve the Worker outbound-only model: they limit inbound access to the Tikeo server Worker Tunnel endpoint but still do not create business Worker inbound Services.
- ServiceMonitor targets the server `/metrics` endpoint for Prometheus Operator installs; Gateway API is optional and requires matching cluster CRDs/controllers.
Verification evidence so far:
- RED/green contract: `python3 -m unittest deploy.tests.iac_artifacts_test.IacArtifactsTest.test_helm_chart_exposes_operational_maturity_contracts` failed before implementation and passed after the templates/schema/docs were added.
- `scripts/verify-deploy-bootstrap.sh` passed.
- `.dev/tools/helm lint deploy/helm/tikeo` passed with only the optional icon recommendation.
- `.dev/tools/helm lint` passed with external DB values and with external DB + TLS + ops hardening + Gateway API values.
- `.dev/tools/helm template` passed for default, external DB, TLS, and ops/Gateway overlays.
- Remote verification: CI run `27129836559` and Coverage run `27129836631` both completed successfully for source commit `e98f6fd7395f1c104050ce8037db79ab5447aed6`.
### 2026-06-08 — Source-size debt cleanup
- Added `scripts/check-source-size.py` as a repo-wide source-size audit for normal Rust/TypeScript/TSX files, excluding generated/dependency/build output.
- Split all known historical >1500-line source files without behavior changes: storage repository tests, workflow runtime methods, migration RBAC role-management migration, server dispatcher processors/tests, registry tests, HTTP part_03 tests, and Web workflow API client functions.
- Current source-size gate is green for the whole repository; future source changes should run the audit before commit.
Verification evidence:
- `python3 scripts/check-source-size.py` passed.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test -p tikeo-storage --all-features` passed.
- `cargo test -p tikeo-server --all-features` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `bun run --cwd web lint` passed.
- `bun run --cwd web typecheck` passed.
- `bun run --cwd web test` passed with 117 tests.
- `bun run --cwd web build` passed with the existing large vendor chunk warning.
- Smoke: `cargo run --bin tikeo -- serve --config /tmp/tikeo-source-size-smoke.toml` plus `curl -fsS http://127.0.0.1:19090/healthz` returned `{"status":"ok","uptime_seconds":0}`.

### 2026-06-08 — Source-size audit CI gate
- Main CI `workflow-policy` now runs `python3 scripts/check-source-size.py`, so files over 1500 lines fail before downstream runtime jobs.
- Workflow contract coverage now asserts the source-size policy gate exists in CI.
Verification evidence:
- RED/green contract: `python3 .github/tests/workflow_contract_test.py -k test_ci_enforces_source_size_before_runtime_jobs` failed before implementation and passed after the CI step was added.
- `python3 scripts/check-source-size.py` passed.
- `python3 .github/tests/workflow_contract_test.py` passed with 11 tests.
- YAML parse for all `.github/workflows/*.yml` passed.
- `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24` passed with 16 external actions and no runtime below node24.
- `git diff --check` passed.

### 2026-06-08 — Standalone docs site scaffold
- `docs/` now contains a Docusaurus 3.10.1 TypeScript documentation site with Bun lockfile, Tikeo homepage, bilingual routing (`en`, `zh-CN`), sidebar IA, Phase A P0 English docs pages, starter Chinese translations, release-note blog entry, and static `llms.txt` / `llms-full.txt` files.
- The scaffold reuses existing Tikeo assets (`tikeo-logo-breathe.gif`, architecture SVGs, console tour GIF) and avoids deployment-provider lock-in.
- Added `.github/tests/docs_site_contract_test.py` to guard the docs scaffold shape.
Verification evidence:
- RED/green contract: `python3 .github/tests/docs_site_contract_test.py` failed before `docs/` existed and passed after implementation.
- `python3 scripts/check-source-size.py` passed.
- `bun install --frozen-lockfile` passed in `docs/`.
- `bun run docs:typecheck` passed in `docs/`.
- `bun run docs:build` passed in `docs/` and generated English plus `zh-CN` output.
- Docs serve smoke passed for `/`, `/docs/`, `/zh-CN/docs/`, `/docs/getting-started/quickstart`, and `/llms.txt` on port `13030`.

### 2026-06-08 — Docs P0 depth and zh-CN route mirror completed
- Current docs P0 set now has evaluation-depth guards and all routes have zh-CN counterparts, preventing the previous Chinese 404 state.
- SDK docs now cover Rust, Go, Java Spring Boot, Python, and Node.js in both English and zh-CN sidebars/content.
- `design/docs-site-build-plan.md` now records Phase B P0 content and Phase C current-route localization as implemented.
- Verified locally with docs contract, source-size audit, Docusaurus typecheck/build, zh-CN route smoke, workflow contract, YAML parse, and git diff check.

### 2026-06-08 — Docs deployment docs made copy-pasteable and zh-CN switching made subpath-safe
- Docusaurus docs now default to standalone root `/` and support GitHub Pages project base `/tikeo/` through `TIKEO_DOCS_URL` / `TIKEO_DOCS_BASE_URL`; this fixes the language-switch 404 class caused by root-relative `/zh-CN/...` links on subpath hosting.
- Deployment docs now cover copy-paste paths for single binary/systemd, Docker Compose SQLite/PostgreSQL/MySQL, Helm/Kubernetes dev/prod/TLS/ops, and runtime configuration parameters.
- Added contract coverage for deployment runbook snippets and baseUrl-safe homepage assets.

### 2026-06-08 — Docs Chinese root path and full Compose files corrected
- Default docs build is root-based again, so independently hosted Chinese routes open at `/zh-CN/...` without requiring `/tikeo/` rewrites.
- GitHub Pages project hosting remains available only when explicitly built with `TIKEO_DOCS_BASE_URL=/tikeo/`.
- Compose deployment docs now include full `docker-compose.yml`, `docker-compose.postgres.yml`, and `docker-compose.mysql.yml` in English and zh-CN.

### 2026-06-08 — Docs locale separation completed
- Docusaurus zh-CN translation resources now cover navbar, footer, docs sidebar, blog options, blog author/tag metadata, and release post content.
- Homepage is locale-aware: `/` remains English by default, `/zh-CN/` is Chinese.
- Locale isolation is now covered by docs contract tests and build/serve smoke.

## 2026-06-08 0.2.0 release preparation

- [x] Release metadata synchronized to `0.2.0` across Cargo workspace, Rust SDK/demo, Java SDK/demo, Python SDK/demo, Node SDK/demo, Web, Website, and Helm chart.
- [x] README / README.zh-CN / Helm README install examples updated from `0.1.0` to `0.2.0`.
- [x] `CHANGELOG.md` contains the `0.2.0` formal release section dated 2026-06-08.
- [x] Release validation passed across core Rust, Web, docs site, Java/Rust/Go/Node/Python SDKs, and Rust/Go/Node/Python/Java demos.

### 2026-06-09 — Cross-language SDK management trigger parity
- Java SDK/demo already had Management API job trigger support; Rust/Go/Python/Node.js SDKs now expose equivalent create+trigger helpers.
- All non-Java SDK trigger helpers default to `triggerType=api` + `executionMode=single` and expose explicit broadcast trigger helpers/selectors to match the server/Web API contract.
- Rust/Go/Python/Node.js worker demos now provide real Management API create+trigger examples under `TIKEO_MANAGEMENT_CREATE_EXAMPLES=1`; Java Boot2/3/4 README files document existing demo controller endpoints.
- `design/tikeo-architecture-design.md` has been updated to mark current Python/Node SDK Worker/demo/management-trigger scope complete.

### 2026-06-09 — Job edit namespace/app migration
- Jobs edit now supports moving a job to a different tenant namespace/app from both HTTP API and Web UI.
- `PATCH /api/v1/jobs/{job}` persists namespace/app changes into `namespace_id`/`app_id`, creates a job version snapshot for scope moves, and checks both source and target scope bindings.
- Canary targets are constrained to the destination namespace/app during job updates; Web create/edit selectors filter canary candidates by selected scope and clear stale selections when scope changes.
- Verification passed: targeted backend/Web RED->GREEN tests, workspace Rust fmt/clippy/test/build, Web typecheck/test/build, source-size audit, and diff whitespace check.

### 2026-06-10 — Docs site CI verification gate
- Main CI now includes a dedicated `Docs site` job after `workflow-policy`.
- The job runs `python3 .github/tests/docs_site_contract_test.py`, `cd docs && bun install --frozen-lockfile`, `bun run docs:typecheck`, and `bun run docs:build`.
- `docs/bun.lock` now uses public `https://registry.npmjs.org/` tarball URLs instead of the previous private Nexus tarball host, so GitHub Actions docs verification does not require private npm proxy credentials.
- Workflow/docs contract tests now guard both the CI job shape and public-registry lockfile requirement.
- Verification passed locally: workflow contract, docs contract, source-size audit, GitHub Actions Node runtime policy, workflow YAML parse, git diff whitespace check, and the full docs Bun install/typecheck/build sequence.

### 2026-06-10 — Source-backed SDK management create+trigger docs
- English and zh-CN SDK docs now include source-backed Management API create+trigger examples for Rust, Go, Java Spring Boot, Python, and Node.js.
- Docs explicitly state SDK management auth uses app-scoped `x-tikeo-api-key` / `TIKEO_MANAGEMENT_API_KEY`, not human OIDC/session credentials.
- Docs and tests preserve the helper contract: API triggers use `triggerType=api`, default `executionMode=single`, and broadcast fan-out is only through explicit helpers/selectors with `broadcastSelector`.
- Java management SDK gained the missing explicit broadcast helper: `BroadcastSelectorRequest` plus `TriggerJobRequest.broadcastApi(...)`.
- Verification passed: RED->GREEN docs contract, RED->GREEN Java helper test, full docs contract, workflow contract, source-size audit, GitHub Actions runtime policy, full Java SDK tests, and Docusaurus frozen install/typecheck/build.

### 2026-06-10 — Management API trigger e2e smoke
- Added `scripts/management-trigger-e2e-smoke.sh`, a repeatable local/CI smoke that starts tikeo with an isolated SQLite DB/config under `.dev/reports/management-trigger-e2e-*`, bootstraps admin auth, seeds namespace/app/worker-pool, creates an app-scoped Service Account + `x-tikeo-api-key`, starts the Node.js demo worker over outbound Worker Tunnel, uses the Node.js SDK `ManagementClient` with `apiJob`/`apiTrigger` to create and trigger a job, then asserts `/api/v1/instances/{id}` reaches `succeeded` with `result.success=true` and `/logs` contains `nodejs demo echo processed`.
- Added `.github/tests/management_smoke_contract_test.py` and workflow contract coverage for the smoke script, repository contract-test CI policy, CI smoke execution, and management-trigger artifact upload.
- Main CI `workflow-policy` now runs repository contract tests, and `other-cross-language-smoke` now runs the management trigger e2e smoke after the cross-language worker parity smoke using the already-built server binary.
- Verification passed: RED->GREEN management smoke contract, RED->GREEN workflow contract for CI wiring, real management trigger e2e smoke, full workflow/docs contracts, source-size audit, GitHub Actions Node runtime policy, YAML parse, `git diff --check`, and Rust workspace fmt/clippy/test/build.

### 2026-06-10 — Source-derived OpenAPI/protobuf reference docs
- Added English and zh-CN reference pages for Management OpenAPI and Worker Tunnel protobuf, derived from `crates/tikeo-server/src/http/openapi.rs`, `crates/tikeo-server/src/http/router.rs`, `crates/tikeo-server/src/http/routes/jobs.rs`, and `crates/tikeo-proto/proto/worker.proto`.
- Linked all Rust, Go, Java Spring Boot, Python, and Node.js SDK management helper docs to exact endpoint anchors for create, trigger, instance polling, instance logs, plus the Worker Tunnel `DispatchTask` message.
- Extended docs contracts so future changes cannot drop reference pages, source tokens, sidebar entries, or SDK helper-to-reference links.
- Fixed an MDX/Docusaurus anchor issue exposed by `docs:build`: endpoint headings now generate stable anchors without broken-anchor warnings.
- Recorded the acceptance-stage rigor/context freshness rule in `~/.codex/CONSTITUTION.md`, OMX project memory/notepad, and `.memory/decisions.md` per user instruction.
Verification evidence:
- RED observed: `python3 .github/tests/docs_site_contract_test.py DocsSiteContractTest.test_reference_docs_are_source_backed_for_openapi_and_worker_proto DocsSiteContractTest.test_sdk_docs_link_helpers_to_exact_reference_anchors` failed because `docs/docs/reference/management-openapi.md` was missing and SDK docs lacked exact reference links.
- `python3 .github/tests/docs_site_contract_test.py DocsSiteContractTest.test_reference_docs_are_source_backed_for_openapi_and_worker_proto DocsSiteContractTest.test_sdk_docs_link_helpers_to_exact_reference_anchors` passed after implementation.
- `python3 .github/tests/workflow_contract_test.py` passed.
- `python3 .github/tests/docs_site_contract_test.py` passed.
- `python3 .github/tests/management_smoke_contract_test.py` passed.
- `python3 scripts/check-source-size.py` passed.
- `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24` passed.
- `.github/workflows/*.yml` YAML parse passed.
- `git diff --check` passed.
- `cd docs && bun install --frozen-lockfile && bun run docs:typecheck && bun run docs:build` passed; build log checked for no `broken anchor` warnings.

### 2026-06-10 — Docs module migration, publishing, search, and user-guide completion
- Migrated the Docusaurus documentation site from `website/` to `docs/`; old `website/` is removed from the build surface.
- Moved previous top-level docs media assets from `docs/assets/` to `assets/docs/`, and updated README links accordingly.
- Added `docs/Dockerfile` plus nginx runtime config modeled after the Web image, and added `.github/workflows/publish-docker-docs.yml` targeting Docker Hub repository `yhyzgn/tikeo-docs`.
- Main CI now validates a docs Docker image build with `push: false`; release setup documents the separate docs image publish lane.
- Completed the pending docs publishing/search/SEO readiness slice: canonical URL/baseUrl metadata, robots.txt, OpenGraph image, search-index.json, updated llms.txt, and llms-full.txt.
- Added English and zh-CN source-backed user guides for Dashboard, Jobs, Instances, Workers, Workflows, Scripts, Audit, and Settings, all guarded by docs contract checks.
- Verification passed: workflow/docs/management-smoke contracts, source-size audit, GitHub Actions Node runtime policy, workflow YAML parse, diff whitespace check, docs frozen install/typecheck/build with no broken-anchor output, `docker build -f docs/Dockerfile docs -t tikeo-docs:local`, and container smoke for `/healthz`, `/docs/`, `/zh-CN/docs/`, `/search/`, `/robots.txt`, and `/search-index.json`.

### 2026-06-10 — Docs acceptance runbooks completed after docs module migration
- Completed the pre-migration docs follow-up that remained after the `website/` -> `docs/` module migration.
- Added source-backed English and zh-CN contributor runbooks for `scripts/management-trigger-e2e-smoke.sh`, including prerequisites, `TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh`, evidence layout, case IDs, and failure triage.
- Added source-backed English and zh-CN Kubernetes controller-specific runbooks for Nginx Ingress, Envoy Gateway, Traefik, and Gateway API, grounded in Helm values/templates and preserving the Worker outbound-only boundary.
- Updated docs sidebar, local search index, `llms.txt`, and `llms-full.txt` so the new runbooks are discoverable.
- Docs contract now guards both runbooks against drifting away from the real smoke script and Helm chart sources.
Verification evidence:
- RED observed: `python3 .github/tests/docs_site_contract_test.py` failed because `deployment/kubernetes-controller-runbook.md` and `deployment/management-trigger-smoke-runbook.md` were missing in English and zh-CN routes.
- `python3 .github/tests/docs_site_contract_test.py` passed after implementation.
- `cd docs && bun run docs:typecheck && bun run docs:build` passed with no `broken anchor` output.
- `TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh` passed; evidence directory `.dev/reports/management-trigger-e2e-20260610T153458Z-230214/`, report `.dev/reports/management-trigger-e2e-20260610T153458Z-230214/management-trigger-e2e-20260610T153458Z-230214.json`.

### 2026-06-10 — Operator-grade docs manual depth completed
- Critical docs are no longer README-adjacent summaries. English overview/installation/quickstart/configuration/SDK pages and zh-CN mirrors now include source-backed operator detail for toolchains, repository surfaces, Owner bootstrap, app-scoped SDK API keys, outbound Worker setup, complete config defaults, SDK dependency coordinates, WorkerConfig defaults, Management clients, and live verification runbooks.
- Added docs contracts to prevent shallow critical pages, nonexistent bootstrap fields, non-exported `TOKEN`, unrunnable `/tmp` Node.js SDK scripts, and nginx port-mapping redirect regressions.
- Verifier found three real quickstart defects; all were fixed and reverified.
- Search and LLM static entrypoints now point at the deeper operator pages.
Verification evidence: docs contract, workflow contract, management-smoke contract, Docusaurus typecheck/build, source-size audit, GitHub Actions runtime policy, diff whitespace check, docs Docker build, and docs container route smoke all passed.

### 2026-06-11 — Notification Center / Alerting boundary plan
- Planned the next alerting evolution as a separate Notification Center rather than a replacement for existing alerting.
- Added `design/notification-center-alerting-plan.md` with source-backed current-state facts, canonical vocabulary, data model, API/UI/runtime migration plan, and acceptance criteria.
- Updated the main architecture design so Alerts own rule/event/silence/recovery semantics while Notifications own reusable channels, templates, policies, delivery, retry, and DLQ.
- Created `.prompt/165-notification-center-alerting-boundary.md` as the next implementation handoff for generic notification channels/policies while keeping existing alert APIs compatible.

### 2026-06-11 — Notification Center implementation and acceptance hardening
- Implemented reusable Notification Center storage/API/runtime/Web/docs foundation while keeping existing Alert APIs compatible.
- Added explicit `notification_channels`, `notification_policies`, `notification_messages`, and `notification_delivery_attempts` migration/entities/repositories with soft-link validation and redaction.
- Added `/api/v1/notification-*` routes/OpenAPI for channel metadata, channel CRUD, policy CRUD/validation, messages, delivery attempts, queue status, and retry-due processing.
- Added Web `/notifications` page with provider metadata, channel/policy CRUD, validation, messages, retry/DLQ queue status, and permission-gated operations.
- Job lifecycle events now materialize reusable notification messages/attempts for success, failed, partial failed, cancelled, retry scheduled, retry exhausted, no eligible worker, and script governance failure.
- Acceptance hardening fixed email secret alias drift, secret header injection coverage, config header redaction, viewer menu consistency, env-only secretRef docs/UI wording, and retry-aware failed vs retry_exhausted semantics.
- Verification passed across targeted RED->GREEN tests, Rust fmt/clippy/test/build, CLI smoke, Web lint/typecheck/test/build, docs typecheck/build, workflow/docs/management contract tests, Node runtime policy, source-size audit, and diff whitespace check.

## 2026-06-11 — Notification Center provider schema/drawer hardening

- Notification channel drawer is now provider-schema driven with linked scope, resource, secret-ref, message type, and template fields.
- Edit mode preserves existing config/secret refs unless replacement toggles are explicitly enabled, preventing redacted API summaries from being written back as real provider config.
- Backend validates channel scope consistency, built-in message types, required template fields, and rejects raw secret config keys for built-in providers.
- Built-in provider coverage now includes Slack Block Kit/attachments/thread, DingTalk action/feed cards and signing, Feishu image/share_chat/card/signing, WeCom voice/template card variants, PagerDuty lifecycle/custom fields, webhook JSON, and email subject/body overrides.

### 2026-06-11 — Notification Center templates and provider schema hardening completed
- Provider-schema driven channel and template editing is implemented for Notification Center, including linked scope/resource/secret selectors and edit-mode preservation of existing provider config/secret refs.
- First-class `notification_templates` are implemented with migration/entity/repository, API CRUD/list/get/delete/render preview, OpenAPI wiring, Web template drawer/preview, and policy template selector.
- Runtime job notification materialization loads enabled stored templates by id or templateKey, performs safe token rendering, stores rendered output under `payload.template`, and provider delivery prefers stored template payloads over channel inline defaults.
- Fresh verification passed: Rust fmt/clippy/test/build, Web lint/typecheck/test/build, docs typecheck/build, docs/workflow/management contracts, GitHub Actions Node runtime policy, source-size audit, and diff whitespace check.


## 2026-06-12 — Docs human operator manual rewrite

- Reworked the Docusaurus `docs/` site from AI/source-note style pages into human-readable operator manuals across English and zh-CN priority pages.
- Strengthened install, quickstart, seed demo data, configuration, SDK integration, deployment, SSE realtime, integrations, troubleshooting, user guide, alerts, notifications, and Notification Center reference docs with step-by-step prerequisites, verification, troubleshooting, and production checklists.
- Notification docs now document the real `channel → template → policy → event → delivery` chain with `CHANNEL_ID` / `TEMPLATE_ID` / `POLICY_ID`, `secretRefs`, `supportsTestSend=false`, retry/DLQ, and the Alerts-vs-Notifications boundary.
- Added docs contract tests that reject public AI handoff wording, `0.0.0.0` client URLs, README-rehash depth, unchainable notification examples, and malformed Notification Center provider tables.
- Final local evidence passed: docs contract/workflow/management smoke tests, source-size audit, whitespace check, `cd docs && bun run docs:typecheck && bun run docs:build`, `docker build -f docs/Dockerfile docs -t tikeo-docs:local`, and docs container smoke on `127.0.0.1:13036` for `/healthz`, `/docs/`, `/zh-CN/docs/`, notification reference, and search index.

## 2026-06-13 — Job notification bindings and message trace acceptance slice

- Added `design/job-notification-bindings-plan.md` and implemented the job-facing notification binding layer over existing Notification Center policies.
- Backend now exposes `/api/v1/jobs/{job}/notification-bindings` CRUD plus `:validate` and `:preview`; bindings require both jobs and notifications permissions, validate enabled channels, validate template/provider compatibility, and preserve job ownership boundaries.
- Job instance notification payloads now include production template context such as job/instance IDs, status, trigger/execution mode, operator fields, worker id, and `logsUrl`.
- Notification Center now exposes `/api/v1/notification-messages/{id}/trace`, including message, policy, delivery attempts, resolved job/instance context, and a redacted latest-log excerpt with tenant-scope guard when job context is resolvable.
- Web Jobs page now has a notification configuration drawer; Notification Center messages now have a detail drawer with delivery and execution-log passthrough.
- Release workflows now sync the workspace version from tag before building server binaries and Docker server images, preventing `tikeo-server` from staying at `0.2.0` after `v0.2.x` tags.

Verification so far:
- `cargo fmt --all -- --check` ✅
- `cargo test -p tikeo-server job_notification_binding_api_compiles_to_job_owned_policy_and_preview --all-features -- --nocapture` ✅
- `cargo test -p tikeo-server notification_message_trace_includes_job_instance_attempts_and_redacted_logs --all-features -- --nocapture` ✅
- `cargo test -p tikeo-server job_notification_binding_validation_rejects_empty_advanced_events_and_provider_mismatch --all-features -- --nocapture` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web src/api/notifications.test.ts src/pages/__tests__/JobsPage.test.tsx src/pages/__tests__/NotificationCenterPage.test.tsx` ✅
- `python3 .github/tests/workflow_contract_test.py` ✅
- `python3 scripts/check-source-size.py` ✅

Pending before release: full workspace clippy/test/build, web/docs lint/build, Docker builds, commit/push/tag, and GitHub Actions monitoring.

## 2026-06-13 — Notification channel drawer UX redesign

- Notification channel create/edit drawer now presents operator-facing structure instead of implementation-order fields: live summary/test left rail plus scoped identity, provider/message shape, credentials, channel parameters/template overrides, and advanced JSON sections.
- Replacement switches are colocated with the credential/config sections they control, preserving explicit edit-mode safety semantics.
- i18n and source-level regression coverage were added for the new layout and copy.
- Local verification passed: web typecheck, lint, full `bun test web/src` (151 passed), production build, source-size audit, and diff whitespace check.

Code review follow-up completed for the drawer redesign: i18n gaps closed, Advanced JSON precedence copy now matches actual merge order, create-mode summary text is no longer edit-mode wording, and payload semantics are protected by `ChannelDrawerPayload.test.ts`.

- 2026-06-13: Reworked notification channel drawer local UI hierarchy into configuration map + domain panels; pending final full build/git checks at time of note.

- 2026-06-13: Refined notification channel drawer typography rhythm and copy density; reduced repeated explanations, normalized scoped text sizes/line heights, and kept changes limited to notification drawer UI.

- 2026-06-13: Completed Notification Center template variable catalog/i18n hardening. Channel and template drawers now provide localized variable labels plus a `?` mapping table, built-in provider metadata exposes all currently supported job/payload variables, and stored template rendering now resolves payload variables like `jobId`, `instanceId`, `operatorName`, `logsUrl`, and `templateKey` against real event context. Local Rust/Web verification passed.

## 2026-06-13 — Notification variable catalog and scope progress UX polish

- Improved the Notification Center channel drawer variable UX for normal laptop resolutions: compact preview chips now open a searchable, segmented, grouped variable map modal with bounded scrolling and card-based placeholder/meaning/example/source presentation.
- Replaced the old left-rail scope ladder boolean active state with a deterministic `channelScopeSteps` progress model: done/current/pending/skipped states now track the right-side Global → Namespace → App → Worker Pool cascade and show localized status labels.
- Added zh-CN/en-US translations for the new variable map filters, empty state, group descriptions, and scope progress labels.
- Added regression coverage for variable catalog Modal/search/group behavior and for the scope progress model.

Verification:
- `bun test web/src/pages/__tests__/NotificationCenterPage.test.tsx web/src/pages/notifications/ChannelDrawerPayload.test.ts web/src/pages/notifications/templateCatalog.test.ts web/src/i18n/i18n.test.ts` ✅ (41 passed)
- `bun test web/src` ✅ (159 passed)
- `bun run --cwd web typecheck` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web build` ✅ (existing Vite chunk-size warning only)
- `python3 scripts/check-source-size.py` ✅
- `git diff --check` ✅

## 2026-06-13 — Notification variable catalog overflow polish

- Fixed the variable preview area so `可用模板变量` renders all variables in a bounded scrollable chip container instead of truncating the list behind a `+N` affordance.
- Fixed variable-map card placeholders so long `{{...}}` names stay on one line and scroll horizontally inside the placeholder area instead of wrapping and breaking card rhythm.
- Removed the variable-map toolbar sticky behavior and raised the Modal z-index to avoid layering/overlap artifacts inside the drawer context.

Verification:
- `bun test web/src/pages/__tests__/NotificationCenterPage.test.tsx web/src/pages/notifications/templateCatalog.test.ts` ✅
- `bun test web/src` ✅ (159 passed)
- `bun run --cwd web typecheck` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web build` ✅ (existing Vite chunk-size warning only)
- `python3 scripts/check-source-size.py` ✅
- `git diff --check` ✅

## 2026-06-13 — Notification channel test result layout polish

- Reworked the channel drawer `保存后测试` result area from a cramped one-column `Descriptions` table into a compact result card.
- The card now separates delivery status, request summary fields, optional error details, and rendered payload preview, improving readability in the left-side drawer rail.
- Added zh-CN/en-US translations for the new test result section labels and regression coverage for the new class structure.

Verification:
- `bun test web/src/pages/__tests__/NotificationCenterPage.test.tsx web/src/i18n/i18n.test.ts` ✅
- `bun test web/src` ✅ (159 passed)
- `bun run --cwd web typecheck` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web build` ✅ (existing Vite chunk-size warning only)
- `python3 scripts/check-source-size.py` ✅
- `git diff --check` ✅

## 2026-06-13 — Docs site human operator manual rebuild

- [x] Docs site IA rebuilt around human operator tasks instead of source-code directory order.
- [x] Added production deployment, SDK/API integration, configuration cookbook, and development/extension manuals with zh-CN mirrors.
- [x] Homepage rewritten as a task-path portal for evaluation, deployment, integration, configuration, notifications, SDKs, development, and troubleshooting.
- [x] Search index and LLM entrypoint surfaces updated for the new manuals.
- [x] Docs typecheck/build, docs contract, source-size, diff-check, Docker image build, and container smoke all passed locally.

## 2026-06-13 — 异常 demo、飞书卡片与公开执行控制台验收

- 多语言 Worker demo 增加并测试 `demo.exception`：Node/Python 抛运行时异常，Go panic，Rust 返回 processor error，Java Spring Boot 2/3/4 抛 `IllegalStateException`；`demo.fail` 保持业务失败语义。
- Node/Python/Go/Rust/Java SDK/Spring adapter 现在在 processor 异常路径把真实异常栈/traceback/backtrace 写入 task log，并返回 failed outcome。
- Feishu/Lark interactive 卡片模板更新为截图风格，失败/成功/普通分别使用红/绿/蓝 header，底部“查看控制台”按钮跳转 `{{consoleUrl}}`。provider metadata 暴露失败、成功、普通三套卡片示例。
- Notification payload 的 `logsUrl` / `consoleUrl` 统一指向 `/public/instances/{id}/console`；新增 `/api/v1/public/job-instances/{id}/trace` 与 Web `/public/instances/:id/console` 免登录执行透传页面，展示消息、投递、实例上下文和脱敏日志。
- Trace 路由拆入 `notification_trace.rs`，保持所有源码文件 <=1500 行。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `bun run --cwd web typecheck && bun run --cwd web lint && bun test --cwd web src && bun run --cwd web build` ✅
- `bun run --cwd docs docs:typecheck && bun run --cwd docs docs:build` ✅
- SDK/demo targeted and full tests for Node/Python/Go/Rust/Java ✅
- `python3 scripts/check-source-size.py` ✅
- `git diff --check` ✅
## 2026-06-13 — 异常 demo、飞书卡片公开控制台收尾补强

- 补强 Notification Center 公开控制台链接：新增 `notification_delivery.public_console_base_url`，未配置时保持 `/public/instances/{id}/console` 相对路径，配置后用于飞书/Lark 等外部办公平台卡片按钮生成绝对 URL。
- Server 主路径、HTTP 管理路径、Worker Tunnel fallback 均复用带 public console base URL 的 `NotificationCenter`，避免不同事件入口生成不一致链接。
- 公开执行透传页面接入现有 i18n，上线中文/英文词典覆盖，避免新增页面硬编码中文。
- config/dev.toml、config/container.toml 与英文/中文 docs 同步新增 public console base URL 配置说明。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `bun run --cwd web typecheck && bun run --cwd web lint && bun test --cwd web src && bun run --cwd web build` ✅
- `bun run --cwd docs docs:typecheck && bun run --cwd docs docs:build` ✅
- SDK 全量：Node/Python/Go/Rust/Java ✅
- Demo 全量：Node/Python/Go/Rust/Spring Boot 2/3/4 ✅
- `python3 scripts/check-source-size.py && git diff --check` ✅

## 2026-06-25 — YAML migration and warning-clean full business regression

- Completed full business-output regression after the dev/tikeo config YAML migration and repository-wide warning cleanup. Regression plan/evidence is under `.dev/reports/full-regression-20260625/` (local, ignored artifact directory).
- Fixed Rust SDK generated protobuf warning root cause in `sdks/rust/tikeo/build.rs` by post-processing generated bindings and documenting generated items/fields, without `#[allow]`/`#[expect]` or lint downgrades. `sdks/rust/tikeo/proto/worker.proto` comments now use proper identifier markup.
- Updated `scripts/migration-cli-full-chain-smoke.sh` to assert the current migration business output: concrete Spring Boot starter dependency `0.3.10`, legacy scheduler keys removed, and minimal worker/management placeholders reserved.
- Business-output smoke evidence passed: notification provider delivery/retry/DLQ, management trigger with Node worker execution and logs, SDK API key lifecycle/scope-deny/redaction/audit, web live routes, migration CLI full chain, Docker compose config, server/web/docs image builds.
- SDK/demo evidence passed: Java SDK, Node SDK, Python SDK/demo, Go SDK/demo, Rust SDK/demo with `RUSTFLAGS=-D warnings`, and Java Spring Boot 2/3/4 worker demos.
- Final hygiene passed: `cargo fmt --all -- --check`; `cargo build --workspace --all-features`; `cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps`; web/docs Bun lint/typecheck/test/build; `python3 scripts/check-source-size.py`; `git diff --check`.
- Suppression scan found no source suppression bypasses; only the red-line rule text in `AGENTS.md` and `prompt.md` matched the exact `#[allow]/#[expect]` pattern.

## 2026-06-25 — Remote CI Rust SDK job follow-up

- Tracked failed remote CI run `28149289282`: only `Rust SDK + demo` failed, first at `cargo fmt --manifest-path sdks/rust/tikeo/Cargo.toml -- --check` for standalone SDK formatting drift.
- Ran the exact CI job locally and fixed root causes without warning suppression: standalone Rust SDK rustfmt drift, `WorkerConfig` `Eq` derive, markdown docs for `ScriptRunner`/`TaskProcessor`, redundant private-module visibility, and `unused_braces` in sandbox tool resolver.
- Exact CI job now passes locally: Rust SDK fmt, clippy `-D warnings`, test, package; Rust worker demo fmt, clippy `-D warnings`, test.
- Hygiene also passes: `python3 scripts/check-source-size.py`, `git diff --check`, and suppression scan only matches the red-line text in `AGENTS.md`/`prompt.md`.
