# tikee 功能覆盖与竞品对照实现复盘清单

> 复盘范围：对照 `design/tikee-architecture-design.md` 中 `2. 功能覆盖与竞品对照`（约第 62-122 行）逐项检查当前代码实现。  
> 复盘日期：2026-05-29  
> 复盘口径：以仓库代码可见实现为准；未做完整端到端运行压测，因此“已覆盖”表示代码路径和主要模型已存在，不等同于生产级充分验证。

## 1. 结论摘要

当前实现已经覆盖了 tikee 的核心骨架：任务 CRUD/API 触发、Cron/FixedRate tick、单机/广播派发、Worker Tunnel、Java/Rust SDK、脚本/wasm 治理模型、工作流定义与部分 DAG 执行、Web 控制台、OpenAPI、RBAC/OIDC/API Token、多租户基础模型、告警、Prometheus/OTLP、审计日志和基础部署材料。

但如果严格按设计文档中 `tikee` 列全部为 ✅ 的目标评估，当前代码**尚未完全覆盖**。主要缺口集中在：完整工作流运行语义、部分处理器类型、脚本/动态执行的生产级沙箱闭环、Calendar/Daily 类高级调度、gRPC/SQL/文件清理等内置处理器、GitOps/IaC 深度能力，以及多租户 secret 全链路隔离。

### 总览统计

| 分类 | 条目数 | ✅ 已覆盖 | 🟡 部分覆盖 | ❌ 未覆盖 | 主要风险 |
|---|---:|---:|---:|---:|---|
| 2.1 调度能力 | 9 | 9 | 0 | 0 | 调度主干已覆盖；生命周期维护/冻结窗口和节假日排除已进入正式 Job schema/API/tick 路径 |
| 2.2 执行模式 | 8 | 8 | 0 | 0 | 广播策略、队列治理、分片恢复、MapReduce reduce 分片、长任务取消/checkpoint、补偿节点、安全表达式和审批 SLA 已补齐主干 |
| 2.3 处理器类型 | 11 | 6 | 5 | 0 | Java/Rust/脚本基础较强；内置 HTTP/gRPC/SQL/文件清理/Webhook 主路径已补齐 |
| 2.4 管理与平台能力 | 10 | 6 | 3 | 1 | 平台能力框架齐全，Web 暗色/移动端基础、租户配额、Secret Store、告警去重/静默已接入，GitOps 等仍部分缺口 |
| **合计** | **38** | **25** | **7** | **6** | **整体为“核心可用、竞品对照仍有高级语义缺口”** |

## 2. 状态定义

- ✅ **已覆盖**：核心数据模型、服务端 API/运行路径、前端或 SDK 使用路径基本齐备。
- 🟡 **部分覆盖**：已有枚举、配置、UI、数据表或部分运行路径，但设计增强项/生产级语义未闭环。
- ❌ **未覆盖**：未发现一等模型或实际执行路径；或仅有占位/文档/示例，不足以算实现。
- ❓ **需运行验证**：代码路径存在，但需要联调/压测/安全测试确认。本文优先归入“部分覆盖”并标注验证风险。

---

## 3. 逐项清单

## 3.1 调度能力（设计文档 2.1）

| 功能 | 当前状态 | 代码证据 | 已实现内容 | 未覆盖/风险 | 建议优先级 |
|---|---|---|---|---|---|
| CRON 表达式 | ✅ 已覆盖 | `ScheduleType::Cron`；`crates/tikee-server/src/tikee.rs` 中 `cron_due` / `parse_cron_expression`；`chrono-tz`；Jobs UI Cron 表单；`cron_tick_uses_iana_timezone_option` / `cron_tick_skips_excluded_calendar_date` 单测 | Server tick loop 支持秒级 Cron；表达式支持 `;tz=IANA` 时区，例如 `;tz=Asia/Shanghai`；支持 `;exclude=YYYY-MM-DD,...` 日历排除；生成 `TriggerType::Cron` | 已接 IANA 时区/DST 解析和日期排除；更复杂节假日 Provider/维护日历集中管理可作为 Calendar 管理增强 | P2 |
| 固定频率 FIX_RATE | ✅ 已覆盖 | `ScheduleType::FixedRate`；`fixed_rate_due`；`parse_fixed_rate_expression`；`MisfirePolicy::LatestOnly`；Jobs UI 支持 fixed_rate/jitter/latest_only；`fixed_rate_expression_*` 与 `fixed_rate_latest_only_misfire_keeps_one_instance` 单测 | Server 统一 tick 可按固定间隔触发；表达式支持 `30s;jitter=5s` 以 job id 确定性抖动分散同频任务；Misfire 支持 fire_once/do_nothing/catch_up_limited/reschedule/latest_only | 可后续把 jitter 策略扩展为租户级/worker-pool 级策略配置 | P2 |
| 固定延迟 FIX_DELAY | ✅ 已覆盖 | `ScheduleType::FixedDelay`；`crates/tikee-server/src/tikee.rs` 的 `fixed_delay_due`；`web/src/pages/JobsPage.tsx` 暴露 fixed_delay | 已基于上次终态实例 `updated_at` + delay 生成下一次触发，首次无历史时可启动一次；Web/API 已可配置 | 指数退避尚未作为独立策略扩展，可后续增强 | P2 |
| API/手动触发 | ✅ 已覆盖 | `crates/tikee-server/src/http/routes/jobs.rs` 的 `/api/v1/jobs/{job}:trigger`；`web/src/pages/JobsPage.tsx` 手动触发/广播触发；Java 管理客户端在 `sdks/java/tikee/.../management` | REST/API 与 Web 手动触发已可用；支持 single/broadcast；支持灰度路由 | 设计中的 gRPC/CLI/EventBridge 统一触发入口未见完整闭环；Webhook 是单独事件入口 | P1 |
| 延迟任务 | ✅ 已覆盖 | `crates/tikee-storage/src/lib.rs` 有 `dispatch_queue.run_after`；`workflow.rs` 的 `workflow_node_run_after`；`workflow_delay_node_uses_run_after_before_materializing` | 派发队列表具备 run_after；工作流 delay 节点按 `config.seconds` 入队延迟，到期后才 materialize 并推进 | 长期 delay queue、near-time cache 分层可作为扩展；取消/重排通过恢复/编辑路径处理 | P2 |
| 一次性未来任务 | ✅ 已覆盖 | `ScheduleType::Once` / `TriggerType::Once`；`once_due`；Jobs UI `once` + RFC3339 触发时间 | 已提供一等 `once` 调度类型，到点后只触发一次；支持 RFC3339 时间（含时区） | 取消与重排通过编辑/禁用任务完成，未另设专用 once API | P2 |
| Daily Time Interval | ✅ 已覆盖 | `ScheduleType::DailyTimeInterval` / `TriggerType::DailyTimeInterval`；`daily_time_interval_due`；`JobRepository::list_enabled_scheduled_jobs`；Jobs UI `daily_time_interval` 表单；`daily_time_interval_tick_*` 单测 | 支持 `HH:MM-HH:MM[/interval]@TZ` 表达式，例如 `09:00-18:00/30m@Asia/Shanghai`；tick 只在每日窗口内、按间隔对齐触发，并避免同一 interval 内重复触发 | 当前支持固定 UTC offset 和 `Asia/Shanghai` 等明确映射；完整 IANA TZ/DST/节假日排除仍归入 Cron/Calendar 增强 | P2 |
| Misfire 策略 | ✅ 已覆盖 | `MisfirePolicy`；jobs/job_versions `misfire_policy`；`misfire_decision`；Jobs UI Misfire 策略选择 | 已支持 `do_nothing`、`fire_once`、`catch_up_limited`、`reschedule`、`latest_only` 并接入 Cron/FixedRate tick | 后续可把 misfire 阈值与 catch-up 上限配置化 | P2 |
| 生命周期窗口 | ✅ 已覆盖 | jobs/job_versions `schedule_start_at` / `schedule_end_at` / `schedule_calendar_json`；HTTP `scheduleCalendar`；`within_lifecycle_window`；`lifecycle_window_blocks_calendar_windows`；Jobs UI 生命周期开始/结束 | 已支持任务级 start/end 调度窗口，时间按 RFC3339 解析；新增正式 `scheduleCalendar` 模型，支持 `maintenanceWindows`、`freezeWindows`、`excludedDates`/`holidays`，tick 决策会在窗口内阻断自动触发 | 更复杂的集中式节假日 Provider 可后续增强 | P2 |

### 调度能力结论

调度层目前已覆盖 API、秒级 Cron（含 IANA 时区/DST 解析与日期排除）、FixedRate（含 jitter 防惊群与 latest-only）、FixedDelay、一次性未来任务、Daily Time Interval、Misfire 主干和任务级生命周期窗口。剩余主要是集中式节假日/维护日历、维护窗口/冻结窗口等治理增强。

---

## 3.2 执行模式（设计文档 2.2）

| 功能 | 当前状态 | 代码证据 | 已实现内容 | 未覆盖/风险 | 建议优先级 |
|---|---|---|---|---|---|
| 单机执行 | ✅ 已覆盖 | `ExecutionMode::Single`；`jobs.rs` trigger；`tunnel/dispatcher.rs` 派发；Worker proto 有 assignment token/task result/log | 单实例任务派发、attempt、Worker lease/assignment token 基础已存在 | 需通过联调继续验证极端重试、幂等 token 的一致性 | P1 |
| 广播执行 | ✅ 已覆盖 | `ExecutionMode::Broadcast`；`BroadcastSelector`；`WorkerRegistry::find_eligible_workers_with_broadcast_selector`；`TriggerJobRequest.broadcastSelector`；Jobs UI 广播抽屉；`registry_matches_broadcast_selector_region_tags_cluster_and_labels` 单测 | 支持按 namespace/app 租户范围广播，并可叠加 structured tags、region、cluster/version、labels 条件筛选；实例详情保留广播子执行模型 | 后续可增加保存/复用广播策略模板，但运行时策略化筛选主干已覆盖 | P2 |
| 分片任务 | ✅ 已覆盖 | `workflow.rs` 的 map/map_reduce shard materialize；`workflow_shards.checkpoint/retry_count`；`rebalance_workflow_shards`；`POST /workflow-instances/{id}/shards/rebalance`；`workflow_failed_shard_rebalance_preserves_checkpoint_and_requeues` 单测 | 工作流 Map/MapReduce 可生成 shard 和队列项；失败 shard 可按 node/status 重平衡重试，保留 checkpoint，重新生成 job instance/dispatch queue | 分片目前绑定工作流节点，不另设一等 job execution mode；策略模板可后续增强 | P2 |
| Map | ✅ 已覆盖 | `workflow.rs` 处理 `map`；`workflow/validation.rs` 校验 map items；`WorkflowsPage.tsx` 提供 Map 节点；shard checkpoint/retry_count/rebalance API | 可定义 map items 并物化为 shard；每个 shard 有 input/output/checkpoint/job_instance_id/retry_count，支持失败分片重试恢复 | 可后续增加动态扩缩分片算法，但 Map 主干与失败恢复已覆盖 | P2 |
| MapReduce | ✅ 已覆盖 | `workflow.rs` 处理 `map_reduce`；`persist_map_reduce_result_chunks`；`workflow.map_reduce.chunk` / `workflow.map_reduce.manifest` 事件；`WorkflowsPage.tsx` MapReduce 节点；`workflow_map_reduce_writes_reduce_chunks_and_manifest` 单测 | 支持 map_reduce 节点定义、shard、完成推进、失败分片 checkpoint/rebalance；全部 shard 成功后按 chunk 写 reduce 结果事件和 manifest，形成结果分片/spill 基础 | 后续可把 chunk size 和外部对象存储 spill 策略配置化 | P2 |
| 工作流 DAG | ✅ 已覆盖 | `workflow.rs` definition/run/advance/materialize/recover；`materialize_next_queued_node_with_fencing` 覆盖 job/script/http/map/map_reduce/sub_workflow/control；`workflow_condition_node_routes_failure_branch_and_auto_advances`；`workflow_condition_node_evaluates_safe_typed_expression`；`workflow_approval_node_times_out_and_routes_failure_branch`；`workflow_compensation_node_auto_advances_after_failure_branch`；`WorkflowsPage.tsx` 可视化编辑/运行/SSE | DAG 定义、校验、运行、job/script/http/map/map_reduce/sub_workflow 节点物化；condition 节点已支持安全受限 typed expression（config/vars 布尔、数字、字符串比较与 `&&`/`||`）；approval 节点支持人工 advance 和 `timeoutSeconds`/`onTimeout` SLA 超时分支；parallel/join/notification/start/end/delay/compensation 控制节点会自动推进；delay 已接 run_after；HTTP 节点物化为内置执行任务实例 | 更复杂的审批升级链路和运行回放可后续增强 | P2 |
| 长运行任务 | ✅ 已覆盖 | Worker Tunnel、heartbeat/lease/generation、assignment token；`dispatcher.rs` stale running recovery；`TaskCheckpoint` proto；`handle_task_checkpoint`；`cancel_job_instance` API；`cancel_job_instance_closes_dispatch_queue` 单测 | 有心跳、租约、重连、stale running 恢复；Worker 可上报 checkpoint 到实例日志；Server 支持取消 pending/running 实例并关闭 dispatch_queue/shard 状态 | Worker 侧主动响应取消命令可后续增强，但 Server 侧 checkpoint/恢复依据/优雅取消主干已覆盖 | P2 |
| 任务排队 | ✅ 已覆盖 | `dispatch_queue` 表含 `priority/run_after/status/lease_owner/lease_until/fencing_token/worker_selector/namespace/app/worker_pool`；`dispatcher.rs` 处理队列、stale 恢复和 WorkerPool quota；metrics 有 queue SLO | 队列、优先级、租约、fencing token、延迟 run_after、stale running 恢复、WorkerPool maxQueueDepth/maxConcurrency 背压和 UI/API 配额管理已存在 | 后续可继续增加更细的租户级权重/公平调度策略 | P1 |

### 执行模式结论

执行模式的“主干”已经搭起来，尤其是 Worker Tunnel、队列、单机/条件广播、工作流分片与失败 shard checkpoint/rebalance、MapReduce reduce 分片 manifest、长任务 checkpoint 与取消。剩余增强点主要集中在工作流安全表达式与审批 SLA。

---

## 3.3 处理器类型（设计文档 2.3）

| 功能 | 当前状态 | 代码证据 | 已实现内容 | 未覆盖/风险 | 建议优先级 |
|---|---|---|---|---|---|
| Java Bean / SDK | ✅ 已覆盖 | `sdks/java/tikee/src/main/java/com/yhyzgn/tikee/processor`；`worker/client/GrpcTikeeWorkerClient.java`；`sdks/java/tikee-spring`；`tikee-spring-boot-starter`；`examples/java/spring-worker-demo` | Java SDK、Spring 注解注册、Starter 自动配置、Worker Tunnel、任务管理客户端、Demo 用例均存在 | 仍需保持 server-java demo 的 E2E 回归测试常态化 | P1 |
| Rust 原生处理器 | ✅ 已覆盖 | `sdks/rust/tikee/src/*`；`examples/rust/worker-demo/src/main.rs` | Rust SDK、Worker 会话、脚本 runner、wasm feature、demo 均存在 | Rust demo 当前可暂缓，但能力模型存在 | P2 |
| Go/Python/Node SDK | 🟡 部分覆盖 | `sdks/go/tikee/*` 有 Go SDK/proto/连接边界；Python/Node 仅见占位/README 级别 | Go 有官方 gRPC 生成/连接与基础边界；Python/Node 未形成完整 SDK | 设计声称三者 ✅，但实际 Go 未达到 Java/Rust parity，Python/Node 未覆盖 | P2 |
| HTTP 调用 | 🟡 部分覆盖 | `WorkflowsPage.tsx` 有 http 节点配置；`workflow/validation.rs` 允许 `http`；`tunnel/dispatcher.rs` 内置 `execute_http_processor`；`http_processor_retries_and_signs_requests` | workflow http 节点已可实际发起 HTTP/HTTPS 调用，支持 method/body/allowedHosts，默认阻断 loopback/private IP，记录实例日志并推进成功/失败；新增 `maxRetries`/`retryBackoffMs` 重试和 `signature` SHA256 签名头，测试验证首轮 500 后重试成功且两次请求均带签名 | 熔断、复杂 denylist/网段策略仍需补强 | P1 |
| Shell/Python/Node/PHP/PowerShell | 🟡 部分覆盖 | `ScriptRunnerKind` Java/Rust；`ScriptSandboxBackend`；`ScriptsPage.tsx`；`crates/tikee-wasm`; Java `WasmScriptRunner/ContainerScriptRunner` | 脚本模型、版本、治理、runner 注册、wasm/container runner 基础存在；Java Spring Starter 默认只注册 Wasmtime/WASM shell，非 WASM 语言需显式容器沙箱配置 | PHP 未见明确支持；Python/JS/TS/PowerShell/Rhai 依赖显式 container sandbox 配置，不再默认本地执行 | P1 |
| SQL 执行 | ✅ 已覆盖 | `workflow/validation.rs` 支持 `sql`；`workflow.rs` 物化内置执行任务；`tunnel/dispatcher.rs` 的 `execute_sql_processor`；`WorkflowsPage.tsx` SQL 节点；`sql_processor_*` 单测 | 工作流 SQL 节点支持 databaseUrl/sql/allowedDatabaseUrls/dryRun/readOnly；默认 dry-run + readOnly；服务端强制 DSN 白名单和 SELECT/EXPLAIN/WITH 只读限制；SQLite SELECT 可真实执行并写实例日志 | Postgres/MySQL 真实执行、参数模板和审批策略可后续增强 | P2 |
| 文件清理 | ✅ 已覆盖 | `workflow/validation.rs` 支持 `file_cleanup`；`workflow.rs` 物化内置执行任务；`tunnel/dispatcher.rs` 的 `execute_file_cleanup_processor`；`WorkflowsPage.tsx` FileClean 节点；`file_cleanup_processor_*` 单测 | 工作流 FileClean 节点支持 paths/allowedRoots/dryRun/recursive；服务端强制 allowedRoots、绝对路径、拒绝 `..`，默认 dry-run，目录删除必须 recursive=true | 可后续补定时清理模板和更细审计字段 | P2 |
| Groovy/动态脚本 | 🟡 部分覆盖 | `ScriptRunnerKind` 支持 shell/python/javascript/typescript/powershell/rhai/wasm 等；脚本版本/审批/签名元数据 UI | 动态脚本体系已存在，Rhai/WASM/JS/TS 等替代 Groovy 能力 | 未见 Groovy 本身；签名/审批/沙箱在部分路径存在，生产强制策略仍需 E2E 验证 | P1 |
| 外部 JAR/容器 | 🟡 部分覆盖 | Java/Rust `ContainerScriptRunner`；plugin registry；script version/wasm binding | 容器 runner 与插件注册具备基础；WASM 优先策略在模型上存在 | 外部 JAR Container 不是一等任务类型；版本化/签名校验对 JAR/容器未完整 | P2 |
| gRPC 调用 | ✅ 已覆盖 | `workflow/validation.rs` 支持 `grpc`；`workflow.rs` 物化内置执行任务；`tunnel/dispatcher.rs` 的 `execute_grpc_processor`；`WorkflowsPage.tsx` gRPC 节点；`grpc_processor_fails_closed_*` 单测 | 工作流 gRPC 节点支持 endpoint/service/method/payload/metadata/allowedHosts；使用 tonic 发起 unary 调用，默认拒绝私网/回环 endpoint，执行结果写实例日志并推进工作流 | 流式 gRPC、服务描述导入、重试/鉴权模板可后续增强 | P2 |
| Webhook | ✅ 已覆盖 | `routes/event_sources.rs` 入站 webhook trigger + HMAC/timestamp/nonce 校验；`alert.rs` 出站 webhook/slack/dingtalk/feishu/wechat_work/pagerduty/email；`inbound_webhook_rejects_replayed_signed_nonce`；`webhook_signature_is_stable` | 入站 webhook 支持签名触发、防 5 分钟外 timestamp、nonce 重放拒绝、payload 日志；出站告警 webhook 有安全 URL 策略和渠道支持 | 可后续补 per-job secret store 与更多 provider 签名模板 | P2 |

### 处理器类型结论

Java/Rust SDK 是当前最成熟部分。脚本/wasm 已有大量基础设施，但“默认强沙箱 + 多语言 + 日志 + 安全策略”的生产闭环仍需严格验证。HTTP/gRPC/文件清理这类设计里列为 ✅ 的处理器，目前不应标记为完成。

---

## 3.4 管理与平台能力（设计文档 2.4）

| 功能 | 当前状态 | 代码证据 | 已实现内容 | 未覆盖/风险 | 建议优先级 |
|---|---|---|---|---|---|
| Web 控制台 | ✅ 已覆盖 | `web/src/pages/*` 覆盖 Jobs/Instances/Workers/Workflows/Scripts/Plugins/Scopes/Alerts/Audit 等；`web/src/theme` 和 AppShell；`ThemeMode.test.ts`；`ResponsiveConsole.test.ts` | 内置 React 控制台、主要管理页面、主题色、分页等已实现；新增可持久化 light/dark 模式，接入 Ant Design `darkAlgorithm`、`data-theme` CSS 和顶栏开关；移动端基础规则覆盖 shell/header/toolbars/table 横向滚动/drawer 全宽 | 仍建议做完整视觉 QA/设备截图验收 | P2 |
| OpenAPI | ✅ 已覆盖 | `crates/tikee-server/src/http/openapi.rs` 使用 `utoipa::OpenApi` 汇总 routes/schema | REST OpenAPI 已生成，覆盖 jobs/workflows/scripts/auth/alerts/metrics 等 | gRPC reflection 未确认 | P2 |
| 实时日志 | ✅ 已覆盖 | `worker.proto` 有 `TaskLog` 和 `SubscribeTaskLogs`；`jobs.rs` 有 instance logs API；Java/Rust SDK 有 task log 上报；UI 实例日志展示；`tunnel::service::tests::subscribe_task_logs_replays_existing_and_streams_live_logs` | gRPC 流式日志、日志持久化查询、历史 replay 与 live stream 已有服务端测试固化；脚本/SDK 日志可进入实例日志 | 对象存储归档属于长期日志归档增强，可后续作为运维扩展；背压压测仍可补充 | P2 |
| 工作流可视化 | 🟡 部分覆盖 | `web/src/pages/WorkflowsPage.tsx` 可视化节点编辑、dry-run/validate/run/SSE；`workflow.rs` 支持定义/运行/恢复 | 拖拽/节点配置、JSON-ish/YAML-ish 文本展示、SSE 事件和恢复入口存在 | YAML/JSON 双模式、diff、仿真、回放不完整；Runtime 节点覆盖不足 | P1 |
| 用户权限 | 🟡 部分覆盖 | `crates/tikee-server/src/http/auth.rs`；OpenAPI 有 OIDC/API token/sdk api keys；RBAC 权限种子在 storage | RBAC、OIDC identity、API token 创建/轮换/撤销、SDK API Key 均存在 | Service Account 是否一等模型不明确；密钥使用审计需补充验证 | P1 |
| 多租户 | ✅ 已覆盖 | `namespaces/apps/worker_pools/sdk_api_keys/secrets` scope；`worker_pools.max_queue_depth/max_concurrency`；`dispatch_queue.namespace/app/worker_pool`；`routes/scope.rs`；`ScopesPage.tsx`；`tenant_secret_store_creates_lists_and_deletes_scoped_secret_refs` | namespace/app/worker pool 基础 CRUD、token scope binding、WorkerPool 队列/并发配额和背压已接入；Secret Store 按 namespace/app 隔离，只存 valueRef，不存明文，创建/删除审计 | 租户级权重/公平调度可后续增强 | P2 |
| 告警通知 | ✅ 已覆盖 | `crates/tikee-server/src/alert.rs`、`alert/email.rs`、`alert/retry.rs`、`routes/alerts.rs`；Web `AlertDeliveryPage`；`alert_rules_apply_threshold_dedupe_window_and_silence` | 邮件、飞书、钉钉、企微、Slack、PagerDuty、Webhook、插件告警、重试/DLQ 基础存在；`dedupe_seconds` 已接入实际窗口化去重/阈值计数，`silenced_until` 会生成 silenced 历史事件且不投递 | 复杂告警表达式、分组聚合和升级策略可后续增强 | P2 |
| 指标监控 | ✅ 已覆盖 | `/metrics` router；`routes/metrics.rs`；`observability/prometheus/*`；`observability/grafana/*`；`observability/tracing.rs` | Prometheus 指标、业务 SLO 汇总、Grafana/Prometheus 配套、OTLP tracing 基础存在 | 指标命名稳定性和 Dashboard 完整性需运维回归 | P2 |
| 审计日志 | 🟡 部分覆盖 | `audit_logs` schema；`routes/audit_logs.rs`；多处 CRUD/trigger/login/script gate 写审计；`AuditLogsPage`；`tenant_secret_store_creates_lists_and_deletes_scoped_secret_refs` 验证 secret create/read/delete 审计；`workflow_approval_advance_records_audit_log` 验证审批推进审计；`user_management_and_rbac_integration` 验证 user create/update/delete 审计；`tenant_scope_management_api_creates_and_lists_namespaces_apps_and_worker_pools` 验证 namespace/app/worker_pool create 与 worker_pool quota update 审计；`tenant_scope_delete_rejects_non_empty_parents_and_deletes_empty_worker_pool` 验证 namespace/app/worker_pool delete 审计 | 审计日志表、查询/导出、多个关键操作审计已存在；Secret Store 创建/读取/删除已写入并测试审计记录；用户 create/update/delete 已通过 audit API 固化；租户范围 namespace/app/worker_pool create/delete 和 worker_pool update 已固化；实例 cancel 已通过 `cancel_job_instance_closes_dispatch_queue`/`cancel_instance_route_records_audit_log` 固化；Job rollback 已通过 `job_version_api_lists_and_rolls_back_snapshots` 的审计断言固化；脚本 publish/rollback 已通过 `script_publish_and_rollback_return_release_pointer_envelopes` 的审计断言固化；工作流审批/advance 已通过 audit API 固化 | 仍需系统性扫描剩余低频管理 CRUD 矩阵是否 100% 覆盖 | P1 |
| GitOps/IaC | ❌ 未覆盖 | `Dockerfile`、`docker-compose.yml`、`deploy/compose`、`deploy/systemd`、`deploy/k8s/tikee.yaml` | 有 Compose/Systemd/K8s baseline 部署材料 | 未见 CRD、Terraform Provider、GitOps diff/import-export 等平台能力；不能按设计标记完成 | P2 |

### 管理与平台能力结论

平台管理能力已具雏形，Web/OpenAPI/Metrics 较完整；但设计中面向企业级治理的“全链路隔离、全量审计、GitOps/IaC、对象存储日志归档”等仍未闭环。

---

## 4. 高优先级缺口清单

### P0：建议优先补齐或降级设计承诺

当前 P0 工作流 Runtime 主干已补齐：condition 安全 typed expression、approval timeout SLA、补偿节点、delay run_after、MapReduce manifest/chunks、shard checkpoint/rebalance 均已有回归测试。

### P1：重要但可排期补强

1. 审计覆盖率检查：剩余 CRUD 矩阵。

### P2：可后续完善

1. Calendar Schedule 的集中式节假日/维护日历管理增强。
2. Go SDK 完整 parity；Python/Node SDK 实现。
4. 外部 JAR Container 一等模型。
5. 暗色模式/移动端验收。
6. CRD/Terraform Provider/GitOps diff。

---

## 5. 代码证据索引

### 设计来源

- `design/tikee-architecture-design.md:62-122`：功能覆盖与竞品对照表。

### 调度与任务

- `crates/tikee-core/src/lib.rs`：`ScheduleType`、`ExecutionMode`、`TriggerType`、脚本/wasm 策略基础类型。
- `crates/tikee-server/src/tikee.rs`：Cron/FixedRate/FixedDelay/Once tick、Misfire 与生命周期窗口。
- `crates/tikee-server/src/http/routes/jobs.rs`：任务 CRUD、trigger、broadcast、versions、rollback、canary routing、instance logs。
- `crates/tikee-storage/src/repository/job_repo.rs`：任务版本、回滚、灰度字段、namespace/app 关联。
- `web/src/pages/JobsPage.tsx`：任务列表、新建/编辑、手动触发、广播触发、Cron/FixedRate 表单、灰度、版本回滚。

### Worker Tunnel、队列与能力匹配

- `crates/tikee-proto/proto/worker.proto`：Worker Tunnel、Worker capabilities、TaskAssignment、TaskResult、TaskLog、SubscribeTaskLogs。
- `crates/tikee-server/src/tunnel/dispatcher.rs`：dispatch_queue 领取、能力路由、SDK/plugin/script/wasm binding、stale running recovery。
- `crates/tikee-server/src/tunnel/capability.rs`：structured capabilities matching。
- `crates/tikee-storage/src/lib.rs`：`dispatch_queue` 表、worker logical instances、namespaces/apps/worker_pools、audit_logs、sdk_api_keys 等 schema。

### 工作流

- `crates/tikee-storage/src/repository/workflow.rs`：workflow definition/run/advance/materialize/recover、map/map_reduce/sub_workflow、dispatch_queue 集成。
- `crates/tikee-storage/src/repository/workflow/validation.rs`：允许的 node kind 及基础校验。
- `web/src/pages/WorkflowsPage.tsx`：工作流可视化编辑、节点配置、dry-run/validate/run/SSE。

### SDK 与 Demo

- `sdks/java/tikee/src/main/java/com/yhyzgn/tikee/worker/client/GrpcTikeeWorkerClient.java`：Java Worker Tunnel 客户端。
- `sdks/java/tikee/src/main/java/com/yhyzgn/tikee/processor/*`：Java processor API 与注解。
- `sdks/java/tikee/src/main/java/com/yhyzgn/tikee/management/*`：Java 管理客户端。
- `sdks/java/tikee-spring/src/main/java/com/yhyzgn/tikee/spring/*`：Spring processor registry/adapter。
- `sdks/java/tikee-spring-boot-starter/src/main/java/com/yhyzgn/tikee/boot/*`：Spring Boot starter 自动配置与生命周期。
- `examples/java/spring-worker-demo/*`：Java Spring Boot demo。
- `sdks/rust/tikee/src/*`：Rust SDK、session、management、script/wasm 支持。
- `examples/rust/worker-demo/src/main.rs`：Rust worker demo。
- `sdks/go/tikee/*`：Go SDK/proto 基础，未达到完整 parity。

### 脚本、WASM、安全执行

- `crates/tikee-wasm/src/lib.rs`：Wasmtime executor、fuel/timeout/memory/no ambient FS/network 校验。
- `sdks/java/tikee/src/main/java/com/yhyzgn/tikee/script/*`：ScriptRunnerKind、ScriptSandboxBackend、WasmScriptRunner、ContainerScriptRunner、LocalSubprocessScriptRunner。
- `web/src/pages/ScriptsPage.tsx`：脚本 CRUD、版本、diff、publish/rollback、审批/签名/策略元数据 UI。

### 管理平台

- `crates/tikee-server/src/http/openapi.rs`：OpenAPI 汇总。
- `crates/tikee-server/src/http/auth.rs`：登录、OIDC、API Token 等。
- `crates/tikee-server/src/http/sdk_api_keys.rs`：SDK API Key 管理。
- `crates/tikee-server/src/http/routes/scope.rs`：namespace/app/worker pool 管理。
- `crates/tikee-server/src/http/routes/audit_logs.rs`：审计日志查询/导出。
- `crates/tikee-server/src/http/routes/metrics.rs`：业务 SLO 与 Prometheus 指标刷新。
- `crates/tikee-server/src/observability/tracing.rs`：OTLP tracing。
- `crates/tikee-server/src/alert.rs`、`alert/email.rs`、`alert/retry.rs`、`http/routes/alerts.rs`：告警渠道、重试、事件与规则 API。
- `web/src/pages/WorkersPage.tsx`、`ScopesPage.tsx`、`PluginsPage.tsx`、`AlertsPage.tsx`、`AuditLogsPage.tsx`：平台治理页面。

### 部署与运维

- `Dockerfile`、`docker-compose.yml`：基础容器化。
- `deploy/compose/*`：Compose 部署。
- `deploy/systemd/*`：Systemd 部署。
- `deploy/bare-metal/*`：裸机部署辅助。
- `deploy/k8s/tikee.yaml`、`deploy/k8s/README.md`：K8s baseline。
- `observability/prometheus/*`、`observability/grafana/*`、`docs/operations/prometheus-grafana-runbook.md`：监控运维材料。

---

## 6. 最终判定

按当前代码实际功能，tikee 已经具备“核心调度平台 + Java/Rust Worker SDK + Web 管理台 + 基础工作流 + 脚本/wasm 治理 + 可观测性”的可联调基础。

但对照设计文档 `2. 功能覆盖与竞品对照` 中 tikee 列的全部 ✅，当前不能判定为完全覆盖。建议在路线图/Phase 清单中把上述 P0/P1 缺口重新标回待办；如果短期不实现，则应把设计文档中的对应 ✅ 改为“规划中/部分覆盖”，避免设计承诺与代码实际能力不一致。
