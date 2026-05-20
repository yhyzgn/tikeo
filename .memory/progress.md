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
- [x] 003-worker-tunnel：server 同时启动 HTTP 9090 与 Worker Tunnel gRPC 9091
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
