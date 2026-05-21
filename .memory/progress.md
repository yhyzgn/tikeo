# 进度记录

## 当前状态

- [x] 架构设计文档完成：`design/scheduler-architecture-design.md`
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
- [x] 001-bootstrap：实现 `scheduler serve`、`/healthz`、`/readyz`
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
- [x] 003-worker-tunnel：新增 `scheduler-proto` crate 与 Worker Tunnel protobuf
- [x] 003-worker-tunnel：实现 server 侧 Worker Tunnel gRPC skeleton 与内存 registry
- [x] 003-worker-tunnel：server 同时启动 HTTP 9090 与 Worker Tunnel gRPC 9998
- [x] 设计路线图标记：gRPC 协议定义与代码生成
- [x] 004-storage-and-scheduler：SeaORM 存储层、SQLite dev DB、MySQL migration feature、Jobs API 持久化
- [x] 005-basic-scheduler：调度领域模型、API 手动触发实例链路、实例列表查询
- [x] 006-worker-sdk-rust-and-java-starter：Rust Worker SDK 注册/心跳客户端 + Java Spring Boot Starter 骨架
- [x] 007-web-ui-foundation：Web 管理端基础工程、Job/Instance 页面骨架
- [x] 008-container-deployment：Docker / Compose / K8s 部署基础
- [x] 009-worker-dispatch：Worker Tunnel 真实任务分发、执行回传与实例状态流转
- [x] 010-scheduler-tick-loop：CRON / Fixed Rate tick loop 与调度触发
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
- Rust Worker SDK 从 `crates/scheduler-worker-sdk` 迁移到 `sdks/rust/scheduler-worker-sdk`，Cargo workspace 显式包含该路径。
- Java Spring Boot Starter SDK 从 `java/` 迁移到 `sdks/java/`；后续已改为 Gradle 验证命令 `./sdks/java/gradlew -p sdks/java test`。
- Dockerfile、README、gitignore、design、prompt 和 memory 中的 SDK 路径引用已同步更新。

## 2026-05-21 041：Dispatch Queue 租约 Claim API
- dispatch_queue 在已有 lease_owner / lease_until 字段基础上新增 repository claim/release 能力：claim 会设置租约 owner、过期时间并递增 attempt。
- 新增 `POST /api/v1/dispatch-queue:claim`，需要 workers manage 权限；成功 claim 会写入 audit log，便于追踪多 server/worker 对队列项的占用。
- 增加存储层测试覆盖 claim、重复 claim 阻止、release 后重新 claim 与 attempt 递增。

## 2026-05-21 042：开发脚本本地访问 URL 覆盖
- 用户手动调整 `scripts/dev.sh`：新增 `SCHEDULER_API_PORT` / `SCHEDULER_WEB_URL` 可配置项。
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
- 规划 `sdks/<language>/<sdk-name>` 结构，Rust SDK 从旧 `sdks/scheduler-worker-sdk` 迁移到 `sdks/rust/scheduler-worker-sdk`。
- Java SDK 规划改为 Gradle 多模块 + JDK 21+，替换 Maven 骨架并统一使用 `./sdks/java/gradlew -p sdks/java test` 验证命令。
- 新增 `examples/<language>/<demo-name>` demo 目录规范；后续开发过程中由 AI 自主判断何时创建 demo 来调试 SDK/Worker/工作流集成链路。

## 2026-05-21 046：SDK 目录整改执行
- Rust Worker SDK 已迁移为 `sdks/rust/scheduler-worker-sdk`，Cargo workspace 已同步；服务端 Dockerfile 已移除 SDK 处理。
- Java SDK 已移除 Maven `pom.xml` 骨架，新增 Gradle Kotlin DSL 多模块构建，统一 JDK 21 toolchain / release。
- 新增 `examples/<language>/<demo-name>` demo 目录骨架与 README；后续 SDK/Worker/工作流调试可按需扩展 runnable demo。
### 2026-05-21 SDK layout correction follow-up
- 用户明确根 `Dockerfile` 只构建 scheduler 服务端；已约束不得复制/缓存/构建 `sdks/` 或 `examples/`。
- SDK 路径规范固定为 `sdks/<language>/<sdk-name>/`，Demo 路径规范固定为 `examples/<language>/<demo-name>/`。
- Rust SDK 路径为 `sdks/rust/scheduler-worker-sdk`；现已移除 repo-local path dependencies，满足独立发布约束。
- 已补齐可独立运行的 Rust demo（`examples/rust/worker-demo`）与 Java Spring Boot demo（`examples/java/spring-worker-demo`）基础。
- Dockerfile 继续保持服务端专用，构建阶段改为 Alpine Rust 镜像并使用 Alpine runtime，避免 SDK/Demo 进入镜像上下文。

### 2026-05-21 Rust SDK independent publishing cleanup
- Removed `sdks/rust/scheduler-worker-sdk` from root Cargo workspace and removed Dockerfile rewrite workaround.
- Made Rust SDK self-contained by bundling `proto/worker.proto`, local `build.rs`, and removing all `../../../crates/*` path dependencies.
- Replaced SDK integration tests with an in-crate mock Worker Tunnel server.

### 2026-05-21 Worker identity assignment cleanup
- Changed Worker Tunnel RegisterWorker payload from client-supplied `worker_id` to optional `client_instance_id`.
- Server registry now generates authoritative `wrk-*` worker ids and returns them in `WorkerRegistered`.
- Rust SDK stores server-assigned worker id after connect and uses it for heartbeat/log/result messages.
- Worker identity cleanup verification completed for Rust workspace, standalone Rust SDK, Java SDK, and Java demo. Java wrapper download hit network EOF once, then verification passed with cached Gradle 8.14 binary.

## 2026-05-21 047：Java SDK Worker Tunnel
- Java Core SDK 新增 protobuf/gRPC 生成，内置 `GrpcSchedulerWorkerClient`，支持 OpenTunnel 注册、读取服务端下发 worker_id、定时心跳、任务日志和任务结果回传。
- Spring Boot Starter auto-configuration 默认创建真实 gRPC client；新增 `scheduler.worker.dry-run` 让 demo/测试无需 live scheduler。
- Java Spring demo 默认 dry-run，可通过配置切换到 live Worker Tunnel。

## 2026-05-21 048：Java SchedulerProcessor 适配
- Spring `SchedulerProcessorRegistry` 已从 bean map 升级为 invocable handler registry，拒绝重复 processor name。
- 新增 `SpringSchedulerTaskProcessor`，当前按 `TaskContext.jobId()` 匹配 `@SchedulerProcessor` 名称，支持 `TaskContext` / `String` / `byte[]` 入参和 `TaskOutcome` / `String` / `boolean` / `void` 返回。
- Spring Boot auto-configuration 已把真实 gRPC client 接到 registry adapter，demo 的 `demo.echo` 可作为真实 processor 方法被调用。
