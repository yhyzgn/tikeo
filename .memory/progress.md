# 进度记录

## 当前状态

- [x] 架构设计文档完成：`design/tikee-architecture-design.md`
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
- [x] 001-bootstrap：实现 `tikee serve`、`/healthz`、`/readyz`
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
- [x] 003-worker-tunnel：新增 `tikee-proto` crate 与 Worker Tunnel protobuf
- [x] 003-worker-tunnel：实现 server 侧 Worker Tunnel gRPC skeleton 与内存 registry
- [x] 003-worker-tunnel：server 同时启动 HTTP 9090 与 Worker Tunnel gRPC 9998
- [x] 设计路线图标记：gRPC 协议定义与代码生成
- [x] 004-storage-and-tikee：SeaORM 存储层、SQLite dev DB、MySQL migration feature、Jobs API 持久化
- [x] 005-basic-tikee：调度领域模型、API 手动触发实例链路、实例列表查询
- [x] 006-worker-sdk-rust-and-java-starter：Rust Worker SDK 注册/心跳客户端 + Java Spring Boot Starter 骨架
- [x] 007-web-ui-foundation：Web 管理端基础工程、Job/Instance 页面骨架
- [x] 008-container-deployment：Docker / Compose / K8s 部署基础
- [x] 009-worker-dispatch：Worker Tunnel 真实任务分发、执行回传与实例状态流转
- [x] 010-tikee-tick-loop：CRON / Fixed Rate tick loop 与调度触发
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
- Rust Worker SDK 从 `crates/tikee` 迁移到 `sdks/rust/tikee`，Cargo workspace 显式包含该路径。
- Java Spring Boot Starter SDK 从 `java/` 迁移到 `sdks/java/`；后续已改为 Gradle 验证命令 `./sdks/java/gradlew -p sdks/java test`。
- Dockerfile、README、gitignore、design、prompt 和 memory 中的 SDK 路径引用已同步更新。

## 2026-05-21 041：Dispatch Queue 租约 Claim API
- dispatch_queue 在已有 lease_owner / lease_until 字段基础上新增 repository claim/release 能力：claim 会设置租约 owner、过期时间并递增 attempt。
- 新增 `POST /api/v1/dispatch-queue:claim`，需要 workers manage 权限；成功 claim 会写入 audit log，便于追踪多 server/worker 对队列项的占用。
- 增加存储层测试覆盖 claim、重复 claim 阻止、release 后重新 claim 与 attempt 递增。

## 2026-05-21 042：开发脚本本地访问 URL 覆盖
- 用户手动调整 `scripts/dev.sh`：新增 `TIKEE_API_PORT` / `TIKEE_WEB_URL` 可配置项。
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
- 规划 `sdks/<language>/<sdk-name>` 结构，Rust SDK 从旧 `sdks/tikee` 迁移到 `sdks/rust/tikee`。
- Java SDK 规划改为 Gradle 多模块 + JDK 21+，替换 Maven 骨架并统一使用 `./sdks/java/gradlew -p sdks/java test` 验证命令。
- 新增 `examples/<language>/<demo-name>` demo 目录规范；后续开发过程中由 AI 自主判断何时创建 demo 来调试 SDK/Worker/工作流集成链路。

## 2026-05-21 046：SDK 目录整改执行
- Rust Worker SDK 已迁移为 `sdks/rust/tikee`，Cargo workspace 已同步；服务端 Dockerfile 已移除 SDK 处理。
- Java SDK 已移除 Maven `pom.xml` 骨架，新增 Gradle Kotlin DSL 多模块构建，统一 JDK 21 toolchain / release。
- 新增 `examples/<language>/<demo-name>` demo 目录骨架与 README；后续 SDK/Worker/工作流调试可按需扩展 runnable demo。
### 2026-05-21 SDK layout correction follow-up
- 用户明确根 `Dockerfile` 只构建 tikee 服务端；已约束不得复制/缓存/构建 `sdks/` 或 `examples/`。
- SDK 路径规范固定为 `sdks/<language>/<sdk-name>/`，Demo 路径规范固定为 `examples/<language>/<demo-name>/`。
- Rust SDK 路径为 `sdks/rust/tikee`；现已移除 repo-local path dependencies，满足独立发布约束。
- 已补齐可独立运行的 Rust demo（`examples/rust/worker-demo`）与 Java Spring Boot demo（`examples/java/spring-worker-demo`）基础。
- Dockerfile 继续保持服务端专用，构建阶段改为 Alpine Rust 镜像并使用 Alpine runtime，避免 SDK/Demo 进入镜像上下文。

### 2026-05-21 Rust SDK independent publishing cleanup
- Removed `sdks/rust/tikee` from root Cargo workspace and removed Dockerfile rewrite workaround.
- Made Rust SDK self-contained by bundling `proto/worker.proto`, local `build.rs`, and removing all `../../../crates/*` path dependencies.
- Replaced SDK integration tests with an in-crate mock Worker Tunnel server.

### 2026-05-21 Worker identity assignment cleanup
- Changed Worker Tunnel RegisterWorker payload from client-supplied `worker_id` to optional `client_instance_id`.
- Server registry now generates authoritative `wrk-*` worker ids and returns them in `WorkerRegistered`.
- Rust SDK stores server-assigned worker id after connect and uses it for heartbeat/log/result messages.
- Worker identity cleanup verification completed for Rust workspace, standalone Rust SDK, Java SDK, and Java demo. Java wrapper download hit network EOF once, then verification passed with cached Gradle 8.14 binary.

## 2026-05-21 047：Java SDK Worker Tunnel
- Java Core SDK 新增 protobuf/gRPC 生成，内置 `GrpcTikeeWorkerClient`，支持 OpenTunnel 注册、读取服务端下发 worker_id、定时心跳、任务日志和任务结果回传。
- Spring Boot Starter auto-configuration 默认创建真实 gRPC client；新增 `tikee.worker.dry-run` 让 demo/测试无需 live tikee。
- Java Spring demo 默认 dry-run，可通过配置切换到 live Worker Tunnel。

## 2026-05-21 048：Java TikeeProcessor 适配
- Spring `TikeeProcessorRegistry` 已从 bean map 升级为 invocable handler registry，拒绝重复 processor name。
- 新增 `SpringTikeeTaskProcessor`，当前按 `TaskContext.jobId()` 匹配 `@TikeeProcessor` 名称，支持 `TaskContext` / `String` / `byte[]` 入参和 `TaskOutcome` / `String` / `boolean` / `void` 返回。
- Spring Boot auto-configuration 已把真实 gRPC client 接到 registry adapter，demo 的 `demo.echo` 可作为真实 processor 方法被调用。

## 2026-05-21 049：Java SDK 三模块重组
- Java SDK 已按用户要求重组为 3 个 Gradle 子模块：`tikee`、`tikee-spring`、`tikee-spring-boot-starter`。
- Spring Framework 的 `@TikeeProcessor` registry/adapter 独立在 `tikee-spring`；Spring Boot Properties/AutoConfiguration/starter 聚合在 `tikee-spring-boot-starter`。
- Java demo 依赖已切换到 `com.yhyzgn.tikee:tikee-spring-boot-starter`。

- Spring Boot Java SDK module renamed to `tikee-spring-boot-starter` per user naming correction; demo dependency updated accordingly.

## 2026-05-21 050：Worker processor key protocol
- Worker Tunnel `DispatchTask` proto 新增 `processor_name` 字段，并同步到服务端 proto、Rust SDK proto、Java SDK proto。
- Server dispatcher 分发任务时填充 `processor_name`，当前兼容性默认等于 `job_id`。
- Rust/Java TaskContext 暴露 processor name；Java Spring adapter 改为优先按 `processorName()` 路由 `@TikeeProcessor`。

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
- Enabled `sqlx-postgres` on `tikee-storage` and migrations so PostgreSQL URLs compile through SeaORM/sqlx.
- Added `config/postgres.toml` with PostgreSQL and CockroachDB URL examples; CockroachDB uses PostgreSQL wire protocol.
- Roadmap marks PostgreSQL + CockroachDB storage support complete at driver/config/template level; live DB smoke remains environment-dependent.

### 2026-05-21 Phase2 cluster coordinator foundation
- Added `tikee-server::cluster` with ClusterCoordinator trait, explicit ClusterMode/ClusterRole, and StandaloneCoordinator.
- `/api/v1/cluster` now reports `role=standalone` with node_id/can_schedule/detail instead of fake `leader`.
- Design now records Raft implementation boundaries: leader ownership gate, follower fencing, DB claim as final idempotency guard, and container-friendly networking.

### 2026-05-21 Phase2 cluster ownership gates
- Tikee tick loop and Worker dispatcher loop now consult `ClusterCoordinator` status before ownership-sensitive work.
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
- Kept current storage-backed no-op coordinator in `tikee-server::cluster`; no new `tikee-cluster` crate yet because runtime boundaries are not stable enough.

### 2026-05-21 Phase2 cluster diagnostics
- Added `/api/v1/cluster/diagnostics` for operator-visible cluster readiness: current status, scheduling gate, persisted Raft metadata, members, transport placeholder, and runtime boundary.
- Chose a separate diagnostics endpoint instead of bloating `/api/v1/cluster`; the lightweight status endpoint stays stable for UI polling.
- Kept cluster runtime in `tikee-server::cluster` for now; no `tikee-cluster` crate until consensus/runtime traits stabilize.

### 2026-05-21 Phase2 dispatch queue fencing token
- Reviewed Phase2: only full Raft runtime remains incomplete; Go/Python SDK stays Phase4.
- Added `dispatch_queue.fencing_token` shape and SQLite compatibility migration; claim responses now include a fencing token.
- Dispatcher now derives a fencing token from ClusterCoordinator status (`standalone:<node>:tikee-dispatcher` today, future `raft:<node>:<leader-token>` when real consensus exists).

### 2026-05-21 Phase2 closeout / Phase3 audit paging
- Consensus dependency direction corrected to TiKV raft-rs (`raft` 0.7.0); full Raft scheduling still stays gated until event-loop/transport/persistence/fencing are real.
- Phase2 distributed safety foundations are documented as complete except real Raft runtime/membership.
- Started Phase3 audit governance by adding server-side audit filters and pagination plus Web UI filter controls.

### 2026-05-21 Phase2 raft-rs correction
- User corrected the OpenRaft direction; project now targets TiKV raft-rs (`raft` crate 0.7.0, Apache-2.0) instead of OpenRaft.
- Added `tikee-server::cluster::raft_rs` bootstrap validation: deterministic string `node_id` -> non-zero u64 raft id, peer voters, `MemStorage + RawNode` construction. This proves dependency/API integration only; no tick loop, campaign, leader token, or scheduling grant exists yet.
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
- Runtime does not campaign, does not wire outbound transport, and still keeps `can_schedule=false` and `leader_fencing_token=null`; tikee ownership remains fenced.
- Next slice: connect validated inbound HTTP messages to the runtime inbox, then implement Ready apply/outbound transport and real leader fencing.

### 2026-05-21 Phase2 raft-rs inbound runtime inbox
- Added a `ClusterCoordinator::submit_raft_message` boundary and wired `RaftRuntimeCoordinator` to enqueue validated `eraftpb::Message` values through a bounded mpsc inbox.
- `/api/v1/raft/append-entries` now returns `accepted=true` only when a running raft-rs runtime inbox accepts the message; standalone or stopped runtimes return `accepted=false` with a clear reason. This still does not grant scheduling ownership or a leader fencing token.
- Next slice: implement outbound peer HTTP transport and Ready apply/state-machine bookkeeping before enabling any leader fencing token.

### 2026-05-21 Phase2 raft-rs outbound transport skeleton
- Added optional `cluster.transport_token` config and `x-tikee-raft-token` support so internal Raft HTTP transport can bypass human session auth without committing production secrets.
- Wired Ready outbound messages through a `RaftPeerTransport` skeleton: raft-rs `Message` values serialize to the existing HTTP wire DTO, base64 payloads are preserved, peer URLs append `/api/v1/raft/append-entries`, and delivery runs asynchronously through reqwest.
- Tikee ownership remains fenced: no campaign, no leader token, no `can_schedule=true`. Next slice is committed-entry apply bookkeeping and fencing-token lifecycle.

### 2026-05-21 End-of-day handoff checkpoint
- Current pushed HEAD before this checkpoint: `222b1d6 Send raft-rs outbound messages through peer HTTP skeleton 📡`; working tree was clean before writing this memory checkpoint.
- Today completed and pushed three Phase2 raft-rs slices: runtime ticker + Ready durability order (`fc67f13`), inbound HTTP -> runtime inbox (`dea7528`), and outbound peer HTTP skeleton + optional `cluster.transport_token` (`222b1d6`).
- Full verification passed after the last code slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build`.
- Next continuation prompt is `.prompt/053-phase2-raft-rs-apply-and-fencing.md`: implement Ready committed-entry apply bookkeeping (`applied_index` persistence), explicitly gate config-change entries, then design leader fencing-token lifecycle. Do not enable `can_schedule=true` from raft role alone.

### 2026-05-22 Phase2 raft-rs apply bookkeeping and fencing lifecycle
- Implemented Ready committed-entry apply bookkeeping using `advance_append` / `advance_apply_to` instead of blindly advancing without state-machine acknowledgement.
- Committed `EntryNormal` entries now monotonically update `raft_metadata.applied_index`; `EntryConfChange` / `EntryConfChangeV2` are explicitly gated and stop apply progress before silent membership mutation.
- Added leader fencing-token lifecycle: only a real raft-rs `Leader` with term > 0 derives `raft:term:<term>:node:<node_id>`, persists it first, then reports `can_schedule=true`; non-leaders clear the token. Tikee/dispatcher gates remain driven by `can_schedule` and dispatcher uses the persisted token.
- Next slice: define business state-machine command envelope/replay idempotency and design dynamic membership handling.
- Full verification passed for this slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build`.

### 2026-05-22 Phase2 raft-rs business command envelope foundation
- Added `raft_applied_commands` no-FK table/entity/repository for idempotent state-machine apply records keyed by `(node_id, log_index)` with `(cluster_id, command_id)` reserved for replay idempotency.
- `EntryNormal` payloads now parse as tikee command envelopes (`command_id`, `command_type`, `payload`). `noop` is applied, unknown command types are recorded as `deferred_unsupported`, invalid JSON is recorded as `rejected`, and apply index still advances deliberately.
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
- The harness now proves a real `campaign()` election can produce exactly one leader, persist `raft:term:1:node:tikee-0`, and set `can_schedule=true` only after the token is persisted.
- Added membership proposal E2E coverage: record proposal intent, propose raft-rs ConfChange, commit/apply it, persist `raft_metadata.conf_state`, mark `raft_membership_proposals` as `applied`, and advance `raft_members` to `active` after committed apply.
- Production Ready handling now mirrors the harness by syncing HardState/log/snapshot/commit into raft-rs `MemStorage` before `advance_append`, keeping RawNode memory state aligned with DB persistence.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server raft_inprocess --all-features`; `cargo test -p tikee-server raft --all-features`.
- Full verification passed for 058: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs restart recovery hardening
- Continued `.prompt/059-phase2-raft-rs-http-transport-e2e-or-persistence-hardening.md` after 058 push.
- Runtime startup now restores raft-rs `MemStorage` from persisted `raft_metadata` and `raft_log_entries`: HardState term/vote/commit plus stored log entries are replayed before the ticker loop starts.
- Startup now clears stale `leader_fencing_token` before runtime observation; scheduling authority must be regenerated from the current real raft-rs role instead of reused after restart.
- Added targeted test `raft_runtime_restore_replays_persisted_metadata_and_clears_stale_fencing` covering restored entries/hardstate and stale token removal.
- Next prompt `.prompt/060-phase2-raft-rs-http-transport-smoke.md` keeps the remaining HTTP/Docker bridge transport smoke as the next Phase2 slice.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server raft_runtime_restore --all-features`; `cargo test -p tikee-server raft --all-features`.
- Full verification passed for 059: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs HTTP transport token smoke
- Continued `.prompt/060-phase2-raft-rs-http-transport-smoke.md` after 059 push.
- Added HTTP route smoke coverage for `/api/v1/raft/append-entries` with `x-tikee-raft-token`: valid internal token bypasses human session auth and enqueues into the raft runtime inbox; invalid token falls back to normal auth and returns an unauthorized standard envelope.
- The test keeps the safety semantics explicit: `accepted=true` means local runtime queue acceptance only, local role remains follower, and no leader fencing token/scheduling authority is granted.
- Updated design roadmap to split completed route-level smoke from the remaining Docker bridge/K8s Service multi-container E2E script.
- Next prompt `.prompt/061-phase2-raft-rs-docker-bridge-e2e-script.md` targets bridge-network script verification without host networking.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server raft_append_entries_internal_token --all-features`.
- Full verification passed for 060: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs Docker bridge E2E script
- Continued `.prompt/061-phase2-raft-rs-docker-bridge-e2e-script.md` after 060 push.
- Added `scripts/raft-bridge-e2e.sh`: builds the tikee server image, creates a Docker bridge network, starts 3 tikee containers with generated raft configs, peers by container DNS (`tikee-N:9090`), and injects `TIKEE__CLUSTER__TRANSPORT_TOKEN` without committing secrets.
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
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server audit_logs_support_server_side_filters_and_pagination --all-features`; `cargo test -p tikee-storage migration_creates_metadata_tables --all-features`.
- Full verification passed for 062: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase3 governed audit JSON export
- Continued `.prompt/063-phase3-audit-export-governance.md` after audit trace/result foundation.
- Added `GET /api/v1/audit-logs:export` with `format=json` only, same actor/action/resource filters as list, `audit:read` permission, stable list ordering via repository, and a 500-row maximum guardrail.
- Export response keeps the standard `{ code, message, data }` envelope and includes governance metadata (`max_rows`, `redacted`, `governance`) plus exported items; CSV is rejected with a clear bad-request message until content-type/redaction rules are designed.
- Added Web audit page “导出 JSON” action that downloads the governed JSON payload for current filters.
- Updated design roadmap and created `.prompt/064-phase3-web-danger-confirm-permission-actions.md` for the next Phase3 UI governance slice.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server audit_logs_support_server_side_filters_and_pagination --all-features`; `cd web && bun run typecheck`.
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
- Added `tikee-core` WASM contract types: `WasmRuntimeKind`, `WasmCapabilities`, `WasmResourcePolicy`, `WasmProcessorSpec`, and `WasmSpecError`.
- Default WASM processor spec selects Wasmtime, `_start`, 30s timeout, 64MiB memory, fuel budget, no network, no preopened host directories, and validates denial of ambient host access.
- Added core tests for stable wire serialization and policy validation.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-core --all-features`.
- Full verification passed for 066: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build` (Vite chunk-size warning unchanged).

### 2026-05-22 Phase3 WASM worker runtime executor
- Started `.prompt/067-phase3-wasm-worker-runtime-executor.md`.
- Added dedicated `crates/tikee-wasm` so Wasmtime remains worker/runtime-boundary only and is not pulled into server HTTP/storage paths.
- Implemented `WasmExecutor` on Wasmtime 45.0.0 with fuel metering, epoch interruption timeout hook, memory cap via StoreLimits, and no WASI ambient imports.
- Added tests for minimal WAT execution, network-capability rejection, missing entrypoint rejection, and fuel exhaustion on a busy loop.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-wasm --all-features`; `cargo clippy -p tikee-wasm --all-targets --all-features -- -D warnings`.
- Full verification passed for 067: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build` (Vite chunk-size warning unchanged).

### 2026-05-22 Phase3 WASM script binding and dispatch metadata
- Started `.prompt/068-phase3-wasm-script-binding-and-dispatch.md`.
- Extended worker proto in server/Rust SDK/Java SDK with `DispatchTask.processor_binding`, `TaskProcessorBinding`, and `WasmProcessorBinding` for dynamic WASM payload + policy metadata.
- Dispatcher now receives `ScriptRepository`; when `processor_name` is `script:<id>`, it loads the script and attaches WASM binding only when `language=wasm`, `status=approved`, and `WasmProcessorSpec` validates default-deny network/filesystem policy.
- Server still does not execute user code; it only passes approved module bytes and policy metadata to connected workers.
- Added dispatcher tests for approved safe WASM binding shape and rejection of draft / network-enabled WASM scripts.
- Rust Worker SDK proto fixture updated and SDK tests passed after regenerated proto.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server tunnel::dispatcher --all-features`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features`.
- Java SDK Gradle test was attempted but first Gradle distribution download was too slow and was stopped; rerun once Gradle is cached.
- Full verification passed for 068: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features`. Java SDK Gradle test was attempted but not completed because the first Gradle distribution download was too slow; rerun once cached.
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
- Full verification passed for 069: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --no-daemon`.
- Known warning: Java Gradle build still reports deprecated features that need Gradle 10 compatibility cleanup.


### 2026-05-22 Phase3 WASM distribution integrity and Gradle 10 cleanup
- Continued `.prompt/070-phase3-wasm-distribution-integrity-and-gradle10-cleanup.md`.
- Extended Worker Tunnel `WasmProcessorBinding` with immutable version hooks (`version_id`, `version_number`), `module_sha256`, and reserved `module_signature` across server proto, Rust SDK proto, and Java SDK proto.
- Script version snapshots now persist `content_sha256`; `ScriptSummary` computes SHA-256 for the current script content without adding database foreign keys.
- Dispatcher includes SHA-256 in WASM bindings and uses matching immutable script version snapshot metadata when available; otherwise it still sends digest-only integrity metadata.
- Rust Worker SDK validates `module_sha256` before Wasmtime compilation/execution and fails digest mismatches clearly.
- Web script management now shows content SHA-256 and WASM sandbox defaults/policy metadata in list/detail/version views.
- Java Gradle protobuf plugin upgraded to 0.10.0 and protoc/grpc artifacts use explicit platform classifier notation, removing Gradle 10 multi-string dependency deprecation warnings under Gradle 9.5.1 `--warning-mode all`.
- Full verification passed for 070: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-22 Phase3 script release pointer and worker version binding
- Continued `.prompt/071-phase3-script-release-pointer-and-worker-version-binding.md` after WASM distribution integrity.
- Added `scripts.released_version_id` / `released_version_number` as soft release pointers to immutable `script_versions` snapshots; no database foreign keys were introduced.
- Fixed script version creation to handle empty version history safely and return constructed summaries without SQLite NULL aggregate decode failures.
- Added repository publish/rollback APIs that move the release pointer and keep current draft content mutable but non-executable.
- Added HTTP `POST /api/v1/scripts/{id}/publish` and `/rollback` endpoints using standard `{code,message,data}` envelopes and audit actions `publish`/`rollback`.
- Dispatcher now fails closed for approved WASM scripts without a release pointer or missing released version, and worker bindings use released snapshot bytes, SHA-256, version id, and version number.
- Web script page now shows released version/id, marks released history rows, and exposes publish/rollback actions under script manage permission.
- Full verification passed for 071: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.
- Known warning: Web build still reports existing >500KB chunk-size warning for large lazily loaded chunks.

### 2026-05-22 Phase3 script policy metadata, runner abstraction, and Web chunk split
- Continued `.prompt/072-phase3-script-policy-engine-and-sandbox-runners.md` and the user-requested Web chunk optimization.
- Added `ScriptExecutionPolicy` in core with resources/network/filesystem/secrets/env metadata and default-deny validation for dangerous grants.
- Persisted policy snapshots on `scripts.policy_json` and immutable `script_versions.policy_json`; compatibility migration uses soft schema changes only and still no database foreign keys.
- HTTP script create/update accepts optional `policy`, rejects network/filesystem/secret grants for now, and returns policy data in the standard envelope.
- Script version diff now includes `policy` changes; Web script management exposes safe resource/env policy fields and policy summaries.
- Rust Worker SDK now has non-WASM `ScriptRunnerKind`, `ScriptRunnerPolicy`, `ScriptRunnerTask`, `ScriptRunner` and `UnsupportedScriptRunner` abstraction; unsupported runner validates default-deny policy and refuses execution until concrete sandbox runners are implemented.
- Web build chunk issue fixed with Vite/Rolldown `codeSplitting.groups` for React/AntD/CodeMirror/utility vendor chunks; `bun run build` no longer emits >500KB chunk warnings.
- Full verification passed for 072: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-22 Phase3 local subprocess script runner foundation
- Continued `.prompt/073-phase3-script-sandbox-runner-implementations.md`.
- Added Rust SDK `LocalSubprocessScriptRunner` as the first opt-in non-WASM runner boundary for Shell/Python/Node/PowerShell/Rhai command mappings.
- Runner validates default-deny policy, requires released immutable version metadata, verifies content SHA-256 before execution, clears inherited env, only forwards whitelisted env vars plus tikee metadata, feeds script through stdin, enforces wall-clock timeout, and caps captured stdout+stderr bytes.
- Added SDK tests for successful shell execution, digest mismatch, missing released snapshot, timeout, output limit, and missing runtime.
- Full verification passed for 073 slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-22 Phase3 non-WASM script runner protocol and UI binding
- Continued `.prompt/074-script-runner-protocol-and-ui-binding.md` after the local subprocess runner foundation.
- Extended Worker Tunnel protocol with `ScriptProcessorBinding` for Shell/Python/Node/PowerShell/Rhai released snapshot payloads while preserving WASM bindings.
- Dispatcher now fails closed unless a script is approved, has a release pointer, resolves to an immutable released `script_versions` row, and the released snapshot itself passes default-deny policy validation.
- Worker selection now honors dynamic script capabilities: `script:wasm` for WASM, `script:<language>` for non-WASM, with explicit `script:*` / `*` wildcards only for controlled pools.
- Rust Worker SDK added `ScriptRunnerRegistry` and executes non-WASM bindings only when the worker explicitly registers a matching runner; missing runners produce a clear failure result.
- Java SDK now explicitly reports unsupported script processor bindings and does not call the normal task processor for them.
- Web script detail drawer now documents required worker capabilities and runtime support for WASM and non-WASM scripts.
- Full verification passed for 074: `cargo fmt --all -- --check`; `cargo test -p tikee-proto --all-features`; `cargo test -p tikee-server --all-features tunnel::dispatcher -- --nocapture`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd web && bun run typecheck`; `cd web && bun test && bun run build`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-22 Phase3 containerized script runner foundation
- Continued `.prompt/075-script-runner-container-and-execution-governance.md` after non-WASM Worker Tunnel protocol binding.
- Added Rust SDK `ContainerScriptRunner` as an explicit Worker-side opt-in runner for non-WASM dynamic scripts.
- Refactored the Rust Worker SDK away from a monolithic `lib.rs`: `lib.rs` now only declares/re-exports modules; implementation moved into `config`, `session`, `task`, `error`, `script`, `wasm`, `proto`, and tests modules, with script runners split into `script/local.rs` and `script/container.rs`.
- The container runner builds Docker-compatible `run --rm -i` commands, passes released script content via stdin, disables container networking with `--network=none`, uses `--read-only`, mounts no host paths, injects tikee metadata env, and forwards only policy-whitelisted env vars.
- Shared released snapshot validation between local subprocess and container runners: language match, version_id/version_number, content SHA-256, default-deny policy, and dangerous network/filesystem/secret rejection before spawn.
- Added deterministic unit tests for container command boundary and pre-runtime dangerous policy rejection; live Docker/K8s smoke and audit/result governance move to 076.
- Full verification passed for 075 after SDK module split: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd web && bun run typecheck && bun test && bun run build`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-22 Project rename to tikee
- Renamed project identity from the previous project identity to tikee across Rust workspace package/crate names, binary name, Docker/Compose/K8s identifiers, config defaults, scripts, docs, memory, and prompts.
- Renamed Rust SDK to `tikee` and Java SDK modules to `tikee`, `tikee-spring`, and `tikee-spring-boot-starter`.
- Changed Java package prefix to `com.yhyzgn.tikee` and updated example imports/application main class.
- Changed worker protobuf package namespace to `tikee.worker.v1` and updated Rust/Java generated-code references.
- Prepared `.prompt/077-script-execution-governance-after-tikee-rename.md` as the next handoff prompt.
- Targeted verification so far: `cargo check --workspace --all-features`; `cargo fmt --all`.

### 2026-05-22 SDK naming contraction
- Applied user-requested SDK naming contraction: Rust SDK previous Rust Worker SDK name -> `tikee`, Java core SDK module previous Java core SDK name -> `tikee`.
- Updated Rust example dependency/imports to use `tikee = { path = "../../../sdks/rust/tikee" }`.
- Updated Java Gradle composite build so `tikee-spring` depends on `project(":tikee")`; Java package prefix remains `com.yhyzgn.tikee`.
- Rename verification fixed one regression: the default admin password text changed to `Tikee@2026!`, so the seeded BCrypt hash was regenerated to match the new credential.
- Full rename verification passed: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.
### 2026-05-23 Phase3 script execution governance visibility
- Continued `.prompt/077-script-execution-governance-after-tikee-rename.md`.
- Added dispatcher-side script governance instance logs for fail-closed dispatch cases and worker capability misses: missing script, not approved, missing release pointer/version, unsupported language, policy rejection, and no eligible `script:<language>`/`script:wasm` worker capability.
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
