# tikeo 功能覆盖与竞品对照实现复盘清单

> 复盘范围：对照 `design/tikeo-architecture-design.md` 中 `2. 功能覆盖与竞品对照`（约第 62-122 行）逐项检查当前代码实现。
> 复盘日期：2026-05-30
> 复盘口径：以仓库代码可见实现为准；未做完整端到端运行压测，因此“已覆盖”表示代码路径和主要模型已存在，不等同于生产级充分验证。

## 1. 结论摘要

当前实现已经覆盖了 tikeo 的核心骨架和本轮 P1/P2 主要功能闭环：任务 CRUD/API 触发、Cron/FixedRate/Daily/Calendar tick、单机/广播派发、Worker Tunnel、Worker 服务集群 master 选举、Java/Rust SDK、脚本/wasm/动态语言治理、工作流 DAG 执行与可视化回放、Web 控制台、OpenAPI、RBAC/OIDC/API Token/Service Account、多租户基础模型、告警、Prometheus/OTLP、审计日志、基础部署材料、GitOps/IaC manifest diff、Terraform Provider 与 K8s CRD controller/operator。

严格按设计文档中 `tikeo` 列全部为 ✅ 的目标评估，Terraform Provider / K8s CRD 的真实控制器或 Provider 实现已补齐。当前仍保留的 P2 缺口集中在非 Java SDK parity（Go/Python/Node 已明确后续）与迁移工具 backlog；迁移工具与非 Java SDK/Demo 按当前任务边界不在本轮实现范围。

### 总览统计

| 分类 | 条目数 | ✅ 已覆盖 | 🟡 部分覆盖 | ❌ 未覆盖 | 主要风险 |
|---|---:|---:|---:|---:|---|
| 2.1 调度能力 | 9 | 9 | 0 | 0 | 调度主干已覆盖；生命周期维护/冻结窗口和节假日排除已进入正式 Job schema/API/tick 路径 |
| 2.2 执行模式 | 8 | 8 | 0 | 0 | 广播策略、队列治理、分片恢复、MapReduce reduce 分片、长任务取消/checkpoint、补偿节点、安全表达式和审批 SLA 已补齐主干 |
| 2.3 处理器类型 | 11 | 10 | 1 | 0 | Java/Rust/脚本/动态语言、外部 JAR/容器和内置 HTTP/gRPC/SQL/文件清理/Webhook 主路径已补齐；非 Java SDK parity 为 P2 |
| 2.4 管理与平台能力 | 10 | 10 | 0 | 0 | 平台能力框架齐全，Web 暗色/移动端基础、租户配额、Secret Store、Service Account、审计、告警去重/静默、GitOps/IaC manifest diff、Terraform Provider 和 K8s CRD controller/operator 已接入 |
| **合计** | **38** | **37** | **1** | **0** | **P1 已清空；Terraform Provider/K8s CRD 控制器已补齐；剩余为非 Java SDK parity（已明确后续）** |

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
| CRON 表达式 | ✅ 已覆盖 | `ScheduleType::Cron`；`crates/tikeo-server/src/tikeo.rs` 中 `cron_due` / `parse_cron_expression`；`chrono-tz`；Jobs UI Cron 表单；`cron_tick_uses_iana_timezone_option` / `cron_tick_skips_excluded_calendar_date` 单测 | Server tick loop 支持秒级 Cron；表达式支持 `;tz=IANA` 时区，例如 `;tz=Asia/Shanghai`；支持 `;exclude=YYYY-MM-DD,...` 日历排除；生成 `TriggerType::Cron` | 后续可接外部节假日 Provider，但核心 Cron 与本地日历策略已覆盖 | P2 |
| 固定频率 FIX_RATE | ✅ 已覆盖 | `ScheduleType::FixedRate`；`fixed_rate_due`；`parse_fixed_rate_expression`；`MisfirePolicy::LatestOnly`；Jobs UI 支持 fixed_rate/jitter/latest_only；`fixed_rate_expression_*` 与 `fixed_rate_latest_only_misfire_keeps_one_instance` 单测 | Server 统一 tick 可按固定间隔触发；表达式支持 `30s;jitter=5s` 以 job id 确定性抖动分散同频任务；Misfire 支持 fire_once/do_nothing/catch_up_limited/reschedule/latest_only | 可后续把 jitter 策略扩展为租户级/worker-pool 级策略配置 | P2 |
| 固定延迟 FIX_DELAY | ✅ 已覆盖 | `ScheduleType::FixedDelay`；`crates/tikeo-server/src/tikeo.rs` 的 `fixed_delay_due`；`web/src/pages/JobsPage.tsx` 暴露 fixed_delay | 已基于上次终态实例 `updated_at` + delay 生成下一次触发，首次无历史时可启动一次；Web/API 已可配置 | 指数退避尚未作为独立策略扩展，可后续增强 | P2 |
| API/手动触发 | ✅ 已覆盖 | `crates/tikeo-server/src/http/routes/jobs.rs` 的 `/api/v1/jobs/{job}:trigger`；`web/src/pages/JobsPage.tsx` 手动触发/广播触发；Java 管理客户端在 `sdks/java/tikeo/.../management` | REST/API 与 Web 手动触发已可用；支持 single/broadcast；支持灰度路由 | gRPC/CLI/EventBridge 统一入口属于后续入口扩展；当前 REST/Web/SDK 手动触发已满足设计主路径 | P2 |
| 延迟任务 | ✅ 已覆盖 | `crates/tikeo-storage/src/lib.rs` 有 `dispatch_queue.run_after`；`workflow.rs` 的 `workflow_node_run_after`；`workflow_delay_node_uses_run_after_before_materializing` | 派发队列表具备 run_after；工作流 delay 节点按 `config.seconds` 入队延迟，到期后才 materialize 并推进 | 长期 delay queue、near-time cache 分层可作为扩展；取消/重排通过恢复/编辑路径处理 | P2 |
| 一次性未来任务 | ✅ 已覆盖 | `ScheduleType::Once` / `TriggerType::Once`；`once_due`；Jobs UI `once` + RFC3339 触发时间 | 已提供一等 `once` 调度类型，到点后只触发一次；支持 RFC3339 时间（含时区） | 取消与重排通过编辑/禁用任务完成，未另设专用 once API | P2 |
| Daily Time Interval | ✅ 已覆盖 | `ScheduleType::DailyTimeInterval` / `TriggerType::DailyTimeInterval`；`daily_time_interval_due`；`JobRepository::list_enabled_scheduled_jobs`；Jobs UI `daily_time_interval` 表单；`daily_time_interval_tick_*` 单测 | 支持 `HH:MM-HH:MM[/interval]@TZ` 表达式，例如 `09:00-18:00/30m@Asia/Shanghai`；tick 只在每日窗口内、按间隔对齐触发，并避免同一 interval 内重复触发 | 当前支持固定 UTC offset 和 `Asia/Shanghai` 等明确映射；完整 IANA TZ/DST/节假日排除仍归入 Cron/Calendar 增强 | P2 |
| Misfire 策略 | ✅ 已覆盖 | `MisfirePolicy`；jobs/job_versions `misfire_policy`；`misfire_decision`；Jobs UI Misfire 策略选择 | 已支持 `do_nothing`、`fire_once`、`catch_up_limited`、`reschedule`、`latest_only` 并接入 Cron/FixedRate tick | 后续可把 misfire 阈值与 catch-up 上限配置化 | P2 |
| 生命周期窗口 | ✅ 已覆盖 | jobs/job_versions `schedule_start_at` / `schedule_end_at` / `schedule_calendar_json`；`CalendarRepository`/`calendars` 表；`routes/calendars.rs`；`CalendarsPage.tsx`；Jobs UI Calendar 选择；`lifecycle_window_blocks_calendar_windows`；`lifecycle_window_resolves_centralized_calendar_ref`；`calendar_management_crud_lists_and_audits_upsert` | 已支持任务级 start/end 调度窗口和集中式 Calendar 管理；Calendar 按 namespace/app/name 维护 excludedDates、holidays、maintenanceWindows、freezeWindows；Job 可通过 `scheduleCalendar.calendarRef` 引用，tick 决策会解析集中式 Calendar 并阻断自动触发 | 后续可接外部节假日 Provider，但集中式本地 Calendar 管理面已闭环 | P2 |

### 调度能力结论

调度层目前已覆盖 API、秒级 Cron（含 IANA 时区/DST 解析与日期排除）、FixedRate（含 jitter 防惊群与 latest-only）、FixedDelay、一次性未来任务、Daily Time Interval、Misfire 主干、任务级生命周期窗口和集中式 Calendar 管理。

---

## 3.2 执行模式（设计文档 2.2）

| 功能 | 当前状态 | 代码证据 | 已实现内容 | 未覆盖/风险 | 建议优先级 |
|---|---|---|---|---|---|
| 单机执行 | ✅ 已覆盖 | `ExecutionMode::Single`；`jobs.rs` trigger；`tunnel/dispatcher.rs` 派发；Worker proto 有 assignment token/task result/log/WorkerClusterElection | 单实例任务派发、attempt、Worker lease/assignment token 基础已存在；single dispatch 已优先 worker domain master，server 调度仍由 Raft leader gate 控制 | 极端重试/幂等 token 可继续做压测回归；当前执行主链路已覆盖 | P2 |
| 广播执行 | ✅ 已覆盖 | `ExecutionMode::Broadcast`；`BroadcastSelector`；`WorkerRegistry::find_eligible_workers_with_broadcast_selector`；`TriggerJobRequest.broadcastSelector`；Jobs UI 广播抽屉；`registry_matches_broadcast_selector_region_tags_cluster_and_labels` 单测 | 支持按 namespace/app 租户范围广播，并可叠加 structured tags、region、cluster/version、labels 条件筛选；实例详情保留广播子执行模型 | 后续可增加保存/复用广播策略模板，但运行时策略化筛选主干已覆盖 | P2 |
| 分片任务 | ✅ 已覆盖 | `workflow.rs` 的 map/map_reduce shard materialize；`workflow_shards.checkpoint/retry_count`；`rebalance_workflow_shards`；`POST /workflow-instances/{id}/shards/rebalance`；`workflow_failed_shard_rebalance_preserves_checkpoint_and_requeues` 单测 | 工作流 Map/MapReduce 可生成 shard 和队列项；失败 shard 可按 node/status 重平衡重试，保留 checkpoint，重新生成 job instance/dispatch queue | 分片目前绑定工作流节点，不另设一等 job execution mode；策略模板可后续增强 | P2 |
| Map | ✅ 已覆盖 | `workflow.rs` 处理 `map`；`workflow/validation.rs` 校验 map items；`WorkflowsPage.tsx` 提供 Map 节点；shard checkpoint/retry_count/rebalance API | 可定义 map items 并物化为 shard；每个 shard 有 input/output/checkpoint/job_instance_id/retry_count，支持失败分片重试恢复 | 可后续增加动态扩缩分片算法，但 Map 主干与失败恢复已覆盖 | P2 |
| MapReduce | ✅ 已覆盖 | `workflow.rs` 处理 `map_reduce`；`persist_map_reduce_result_chunks`；`workflow.map_reduce.chunk` / `workflow.map_reduce.manifest` 事件；`WorkflowsPage.tsx` MapReduce 节点；`workflow_map_reduce_writes_reduce_chunks_and_manifest` 单测 | 支持 map_reduce 节点定义、shard、完成推进、失败分片 checkpoint/rebalance；全部 shard 成功后按 chunk 写 reduce 结果事件和 manifest，形成结果分片/spill 基础 | 后续可把 chunk size 和外部对象存储 spill 策略配置化 | P2 |
| 工作流 DAG | ✅ 已覆盖 | `workflow.rs` definition/run/advance/materialize/recover；`materialize_next_queued_node_with_fencing` 覆盖 job/script/http/map/map_reduce/sub_workflow/control；`workflow_condition_node_routes_failure_branch_and_auto_advances`；`workflow_condition_node_evaluates_safe_typed_expression`；`workflow_approval_node_times_out_and_routes_failure_branch`；`workflow_compensation_node_auto_advances_after_failure_branch`；`WorkflowsPage.tsx` 可视化编辑/运行/SSE | DAG 定义、校验、运行、job/script/http/map/map_reduce/sub_workflow 节点物化；condition 节点已支持安全受限 typed expression（config/vars 布尔、数字、字符串比较与 `&&`/`||`）；approval 节点支持人工 advance 和 `timeoutSeconds`/`onTimeout` SLA 超时分支；parallel/join/notification/start/end/delay/compensation 控制节点会自动推进；delay 已接 run_after；HTTP 节点物化为内置执行任务实例 | 更复杂的审批升级链路和运行回放可后续增强 | P2 |
| 长运行任务 | ✅ 已覆盖 | Worker Tunnel、heartbeat/lease/generation、assignment token、WorkerClusterElection/master state；`dispatcher.rs` stale running recovery；`TaskCheckpoint` proto；`handle_task_checkpoint`；`cancel_job_instance` API；`cancel_job_instance_closes_dispatch_queue` 单测 | 有心跳、租约、重连、stale running 恢复；Worker 可上报 checkpoint 到实例日志；Server 支持取消 pending/running 实例并关闭 dispatch_queue/shard 状态；worker domain 可自主确定唯一 master 并故障切换 | Worker 侧主动响应取消命令可后续增强，但 Server 侧 checkpoint/恢复依据/优雅取消主干已覆盖 | P2 |
| 任务排队 | ✅ 已覆盖 | `dispatch_queue` 表含 `priority/run_after/status/lease_owner/lease_until/fencing_token/worker_selector/namespace/app/worker_pool`；`dispatcher.rs` 处理队列、stale 恢复和 WorkerPool quota；metrics 有 queue SLO | 队列、优先级、租约、fencing token、延迟 run_after、stale running 恢复、WorkerPool maxQueueDepth/maxConcurrency 背压和 UI/API 配额管理已存在 | 更细的租户级权重/公平调度策略属于后续优化 | P2 |

### 执行模式结论

执行模式的“主干”已经搭起来，尤其是 Worker Tunnel、队列、单机/条件广播、工作流分片与失败 shard checkpoint/rebalance、MapReduce reduce 分片 manifest、长任务 checkpoint 与取消。剩余增强点主要集中在工作流安全表达式与审批 SLA。

---

## 3.3 处理器类型（设计文档 2.3）

| 功能 | 当前状态 | 代码证据 | 已实现内容 | 未覆盖/风险 | 建议优先级 |
|---|---|---|---|---|---|
| Java Bean / SDK | ✅ 已覆盖 | `sdks/java/tikeo/src/main/java/net/tikeo/processor`；`worker/client/GrpcTikeoWorkerClient.java`；`sdks/java/tikeo-spring`；`tikeo-spring-boot-starter`；`examples/java/spring-boot3-worker-demo` | Java SDK、Spring 注解注册、Starter 自动配置、Worker Tunnel、任务管理客户端、Demo 用例均存在 | server-java demo E2E 应常态化纳入回归，但功能项已覆盖 | P2 |
| Rust 原生处理器 | ✅ 已覆盖 | `sdks/rust/tikeo/src/*`；`examples/rust/worker-demo/src/main.rs` | Rust SDK、Worker 会话、脚本 runner、wasm feature、demo 均存在 | Rust demo 当前可暂缓，但能力模型存在 | P2 |
| Go/Python/Node SDK | 🟡 部分覆盖 | `sdks/go/tikeo/*` 有 Go SDK/proto/连接边界；Python/Node 仅见占位/README 级别 | Go 有官方 gRPC 生成/连接与基础边界；Python/Node 未形成完整 SDK | 设计声称三者 ✅，但实际 Go 未达到 Java/Rust parity，Python/Node 未覆盖 | P2 |
| HTTP 调用 | ✅ 已覆盖 | `WorkflowsPage.tsx` 有 http 节点配置；`workflow/validation.rs` 允许 `http`；`tunnel/dispatcher.rs` 内置 `execute_http_processor`；`http_processor_retries_and_signs_requests`；`http_processor_enforces_denylist_and_circuit_breaker` | workflow http 节点已可实际发起 HTTP/HTTPS 调用，支持 method/body/allowedHosts/deniedHosts/deniedCidrs，默认阻断 loopback/private IP，记录实例日志并推进成功/失败；支持 `maxRetries`/`retryBackoffMs` 重试、`signature` SHA256 签名头、CIDR/通配主机 denylist 和 `circuitBreaker.failureThreshold` 熔断；Web 节点配置已暴露治理字段 | 后续可补 DNS 解析后 CIDR 校验和跨实例全局熔断统计，但当前设计条目中的 HTTP 调用治理已闭环 | P2 |
| Shell/Python/Node/PHP/PowerShell | ✅ 已覆盖 | `ScriptLanguage`；Java `ScriptRunnerKind`；`ScriptSandboxBackend`；`ScriptsPage.tsx`；`crates/tikeo-wasm`; Java `WasmScriptRunner/ContainerScriptRunner`；`enablingSandboxScriptsAdvertisesScriptCapabilitiesWhenRuntimeCheckIsDisabled` | 脚本模型、版本、治理、runner 注册、wasm/container runner 基础存在；Shell 默认走 Wasmtime/WASM shell，Python/JavaScript/TypeScript/PowerShell/PHP/Groovy/Rhai 可通过显式容器沙箱镜像注册能力，且 UI 可创建对应语言脚本 | PHP/Groovy 等非 WASM 语言依赖运维配置容器镜像，符合“显式沙箱启用”策略 | P2 |
| SQL 执行 | ✅ 已覆盖 | `workflow/validation.rs` 支持 `sql`；`workflow.rs` 物化内置执行任务；`tunnel/dispatcher.rs` 的 `execute_sql_processor`；`WorkflowsPage.tsx` SQL 节点；`sql_processor_*` 单测 | 工作流 SQL 节点支持 databaseUrl/sql/allowedDatabaseUrls/dryRun/readOnly；默认 dry-run + readOnly；服务端强制 DSN 白名单和 SELECT/EXPLAIN/WITH 只读限制；SQLite SELECT 可真实执行并写实例日志 | Postgres/MySQL 真实执行、参数模板和审批策略可后续增强 | P2 |
| 文件清理 | ✅ 已覆盖 | `workflow/validation.rs` 支持 `file_cleanup`；`workflow.rs` 物化内置执行任务；`tunnel/dispatcher.rs` 的 `execute_file_cleanup_processor`；`WorkflowsPage.tsx` FileClean 节点；`file_cleanup_processor_*` 单测 | 工作流 FileClean 节点支持 paths/allowedRoots/dryRun/recursive；服务端强制 allowedRoots、绝对路径、拒绝 `..`，默认 dry-run，目录删除必须 recursive=true | 可后续补定时清理模板和更细审计字段 | P2 |
| Groovy/动态脚本 | ✅ 已覆盖 | `ScriptLanguage::Groovy`；Java `ScriptRunnerKind.GROOVY`；Starter `tikeo.worker.scripts.images.groovy`；`ScriptsPage.tsx` 语言选项；脚本版本/审批/签名元数据 UI | 动态脚本体系已存在；Groovy 已是一等语言枚举和 Java Worker 容器沙箱能力，Rhai/WASM/JS/TS 等动态脚本也可用，脚本版本、审批、签名治理在服务端/UI 保持统一 | Groovy 本地裸执行不作为默认路径；必须显式容器沙箱镜像，避免非沙箱执行 | P2 |
| 外部 JAR/容器 | ✅ 已覆盖 | `PluginProcessorTypeSummary` 的 `artifactRef/containerImage/entrypoint/checksum`；`routes/plugins.rs` external_jar 校验；`PluginsPage.tsx` 外部 JAR/容器注册表单；`plugin_registry_supports_custom_processor_types_and_alert_channels` | 外部 JAR/容器已作为 `external_jar` 插件处理器类型建模，可声明版本化 artifactRef、执行镜像、entrypoint、checksum，并通过任务 `processorType=external_jar` + processorName 绑定 Worker pluginProcessors 能力 | 后续可增强真实镜像拉取/签名验证策略，但调度与治理模型已闭环 | P2 |
| gRPC 调用 | ✅ 已覆盖 | `workflow/validation.rs` 支持 `grpc`；`workflow.rs` 物化内置执行任务；`tunnel/dispatcher.rs` 的 `execute_grpc_processor`；`WorkflowsPage.tsx` gRPC 节点；`grpc_processor_fails_closed_*` 单测 | 工作流 gRPC 节点支持 endpoint/service/method/payload/metadata/allowedHosts；使用 tonic 发起 unary 调用，默认拒绝私网/回环 endpoint，执行结果写实例日志并推进工作流 | 流式 gRPC、服务描述导入、重试/鉴权模板可后续增强 | P2 |
| Webhook | ✅ 已覆盖 | `routes/event_sources.rs` 入站 webhook trigger + HMAC/timestamp/nonce 校验；`alert.rs` 出站 webhook/slack/dingtalk/feishu/wechat_work/pagerduty/email；`inbound_webhook_rejects_replayed_signed_nonce`；`webhook_signature_is_stable` | 入站 webhook 支持签名触发、防 5 分钟外 timestamp、nonce 重放拒绝、payload 日志；出站告警 webhook 有安全 URL 策略和渠道支持 | 可后续补 per-job secret store 与更多 provider 签名模板 | P2 |

### 处理器类型结论

Java/Rust SDK 是当前最成熟部分。脚本/wasm、HTTP/gRPC/SQL/文件清理、外部 JAR/容器和 Webhook 均已进入正式模型/API/运行路径；后续重点是非 Java SDK parity 与运行时压测/安全回归。

---

## 3.4 管理与平台能力（设计文档 2.4）

| 功能 | 当前状态 | 代码证据 | 已实现内容 | 未覆盖/风险 | 建议优先级 |
|---|---|---|---|---|---|
| Web 控制台 | ✅ 已覆盖 | `web/src/pages/*` 覆盖 Jobs/Instances/Workers/Workflows/Scripts/Plugins/Scopes/Alerts/Audit 等；`web/src/theme` 和 AppShell；`ThemeMode.test.ts`；`ResponsiveConsole.test.ts` | 内置 React 控制台、主要管理页面、主题色、分页等已实现；新增可持久化 light/dark 模式，接入 Ant Design `darkAlgorithm`、`data-theme` CSS 和顶栏开关；移动端基础规则覆盖 shell/header/toolbars/table 横向滚动/drawer 全宽 | 仍建议做完整视觉 QA/设备截图验收 | P2 |
| OpenAPI | ✅ 已覆盖 | `crates/tikeo-server/src/http/openapi.rs` 使用 `utoipa::OpenApi` 汇总 routes/schema | REST OpenAPI 已生成，覆盖 jobs/workflows/scripts/auth/alerts/metrics 等 | gRPC reflection 未确认 | P2 |
| 实时日志 | ✅ 已覆盖 | `worker.proto` 有 `TaskLog` 和 `SubscribeTaskLogs`；`jobs.rs` 有 instance logs API；Java/Rust SDK 有 task log 上报；UI 实例日志展示；`tunnel::service::tests::subscribe_task_logs_replays_existing_and_streams_live_logs` | gRPC 流式日志、日志持久化查询、历史 replay 与 live stream 已有服务端测试固化；脚本/SDK 日志可进入实例日志 | 对象存储归档属于长期日志归档增强，可后续作为运维扩展；背压压测仍可补充 | P2 |
| 工作流可视化 | ✅ 已覆盖 | `web/src/pages/WorkflowsPage.tsx` 可视化节点编辑、JSON/YAML/定义 Diff、dry-run/validate/run/SSE、server-side replay；`web/src/pages/__tests__/WorkflowsPage.test.tsx` 固化回放与 Diff 入口；`workflow.rs` 支持定义/运行/恢复/回放查询 | 拖拽/端口连线/节点属性、JSON/YAML 双视图、定义 Diff、Dry-run 仿真、SSE 事件流、运行快照回放和 Runtime 状态叠加均已在 Web 控制台闭环 | 复杂历史版本之间的图形化三方合并可作为后续增强，不影响当前设计条目覆盖 | P2 |
| 用户权限 | ✅ 已覆盖 | `crates/tikeo-server/src/http/auth.rs`；`crates/tikeo-server/src/http/sdk_api_keys.rs`；`crates/tikeo-storage/src/entities/sdk_api_key.rs`；OpenAPI 有 OIDC/API token/sdk api keys；`ApiKeysPage.tsx`；`sdk_api_key_lifecycle_uses_header_and_app_scope` | RBAC、OIDC identity、API token 创建/轮换/撤销、SDK API Key 均存在；Service Account 已是独立 app-scoped 机器身份资源，SDK Key 正式绑定 `service_account_id/service_account_name`，鉴权主体为 `service_account:<id>`，角色包含 `service_account`，并按 Service Account 当前 namespace/app 与 key scopes 限权；身份创建/更新/禁用、密钥创建/更新/吊销/使用审计已固化，禁用 Service Account 会吊销关联 active key | 密钥轮换策略可后续增强；当前产品决策为编辑不换 key，需要换 key 时新建后吊销旧 key | P2 |
| 多租户 | ✅ 已覆盖 | `namespaces/apps/worker_pools/sdk_api_keys/secrets` scope；`worker_pools.max_queue_depth/max_concurrency`；`dispatch_queue.namespace/app/worker_pool`；`routes/scope.rs`；`ScopesPage.tsx`；`tenant_secret_store_creates_lists_and_deletes_scoped_secret_refs` | namespace/app/worker pool 基础 CRUD、token scope binding、WorkerPool 队列/并发配额和背压已接入；Secret Store 按 namespace/app 隔离，只存 valueRef，不存明文，创建/删除审计 | 租户级权重/公平调度可后续增强 | P2 |
| 告警通知 | ✅ 已覆盖 | `crates/tikeo-server/src/alert.rs`、`alert/email.rs`、`alert/retry.rs`、`routes/alerts.rs`；Web `AlertDeliveryPage`；`alert_rules_apply_threshold_dedupe_window_and_silence` | 邮件、飞书、钉钉、企微、Slack、PagerDuty、Webhook、插件告警、重试/DLQ 基础存在；`dedupe_seconds` 已接入实际窗口化去重/阈值计数，`silenced_until` 会生成 silenced 历史事件且不投递 | 复杂告警表达式、分组聚合和升级策略可后续增强 | P2 |
| 指标监控 | ✅ 已覆盖 | `/metrics` router；`routes/metrics.rs`；`observability/prometheus/*`；`observability/grafana/*`；`observability/tracing.rs` | Prometheus 指标、业务 SLO 汇总、Grafana/Prometheus 配套、OTLP tracing 基础存在 | 指标命名稳定性和 Dashboard 完整性需运维回归 | P2 |
| 审计日志 | ✅ 已覆盖 | `audit_logs` schema；`routes/audit_logs.rs`；多处 CRUD/trigger/login/script gate 写审计；`AuditLogsPage`；`tenant_secret_store_creates_lists_and_deletes_scoped_secret_refs`；`workflow_approval_advance_records_audit_log`；`user_management_and_rbac_integration`；`tenant_scope_management_api_creates_and_lists_namespaces_apps_and_worker_pools`；`tenant_scope_delete_rejects_non_empty_parents_and_deletes_empty_worker_pool`；`sdk_api_key_lifecycle_uses_header_and_app_scope` | 审计日志表、查询/导出和关键管理/执行操作审计已存在；Secret Store 创建/读取/删除、用户 create/update/delete、租户范围 create/update/delete、实例 cancel、Job rollback、脚本 publish/rollback、工作流审批/advance、SDK API-Key create/update/revoke/use 均已通过 audit API 固化 | 后续新增管理路由必须同步补审计断言；当前设计矩阵中的核心审计覆盖已闭环 | P2 |
| GitOps/IaC | ✅ 已覆盖 | `routes/gitops.rs`；`GET /api/v1/gitops/manifest`；`POST /api/v1/gitops/diff`；`GitOpsPage.tsx`；`deploy/gitops/tikeo-manifest.example.yaml`；`deploy/k8s/crd/tikeo-manifest-crd.yaml`；`deploy/k8s/operator/*`；`deploy/terraform/provider/*`；`deploy/terraform/tikeo_gitops_manifest.tf`；`gitops_manifest_exports_yaml_and_reports_drift_diff`；`deploy/tests/iac_artifacts_test.py` | 已实现 Job/Workflow/Script/Plugin/AlertRule 声明式 Manifest 导出、YAML/JSON、canonical checksum、desired manifest drift diff、Web GitOps/IaC 页面、Terraform Plugin Framework provider（`tikeo_manifest` data source + `tikeo_manifest_diff` resource）和 K8s `TikeoManifest` CRD reconciler/operator status 闭环 | Terraform/K8s apply 仍保持 review-first：批量 mutation 不绕过 typed CRUD、RBAC、审批和审计链；真实集群长期 watch/leader election 可作为部署增强 | P2 |

### 管理与平台能力结论

平台管理能力已具雏形，Web/OpenAPI/Metrics/工作流可视化/RBAC、Service Account、GitOps/IaC manifest diff、Terraform Provider 与 K8s CRD controller/operator 已闭环。对象存储日志归档、复杂租户公平调度等可作为后续运维增强。

---

## 4. 高优先级缺口清单

### P0：建议优先补齐或降级设计承诺

当前 P0 工作流 Runtime 主干已补齐：condition 安全 typed expression、approval timeout SLA、补偿节点、delay run_after、MapReduce manifest/chunks、shard checkpoint/rebalance 均已有回归测试。

### P1：重要但可排期补强

当前 P1 已清空；本轮已补齐工作流可视化回放/Diff、Service Account、HTTP processor 治理、动态脚本语言矩阵和审计矩阵断言。

### P2：可后续完善

1. Go SDK 完整 parity；Python/Node SDK 实现。
2. Terraform Provider/K8s operator 的发布工程增强：registry 发布、镜像构建、controller-runtime manager watch/leader election 与真实集群 e2e。

---

## 5. 代码证据索引

### 设计来源

- `design/tikeo-architecture-design.md:62-122`：功能覆盖与竞品对照表。

### 调度与任务

- `crates/tikeo-storage/src/repository/calendar.rs`、`crates/tikeo-server/src/http/routes/calendars.rs`、`web/src/pages/CalendarsPage.tsx`：集中式 Calendar 管理。

- `crates/tikeo-core/src/lib.rs`：`ScheduleType`、`ExecutionMode`、`TriggerType`、脚本/wasm 策略基础类型。
- `crates/tikeo-server/src/tikeo.rs`：Cron/FixedRate/FixedDelay/Once tick、Misfire 与生命周期窗口。
- `crates/tikeo-server/src/http/routes/jobs.rs`：任务 CRUD、trigger、broadcast、versions、rollback、canary routing、instance logs。
- `crates/tikeo-storage/src/repository/job_repo.rs`：任务版本、回滚、灰度字段、namespace/app 关联。
- `web/src/pages/JobsPage.tsx`：任务列表、新建/编辑、手动触发、广播触发、Cron/FixedRate 表单、灰度、版本回滚。

### Worker Tunnel、队列与能力匹配

- `crates/tikeo-proto/proto/worker.proto`：Worker Tunnel、Worker capabilities、TaskAssignment、TaskResult、TaskLog、SubscribeTaskLogs。
- `crates/tikeo-server/src/tunnel/dispatcher.rs`：dispatch_queue 领取、能力路由、SDK/plugin/script/wasm binding、stale running recovery。
- `crates/tikeo-server/src/tunnel/capability.rs`：structured capabilities matching。
- `crates/tikeo-storage/src/lib.rs`：`dispatch_queue` 表、worker logical instances、namespaces/apps/worker_pools、audit_logs、sdk_api_keys 等 schema。

### 工作流

- `crates/tikeo-storage/src/repository/workflow.rs`：workflow definition/run/advance/materialize/recover、map/map_reduce/sub_workflow、dispatch_queue 集成。
- `crates/tikeo-storage/src/repository/workflow/validation.rs`：允许的 node kind 及基础校验。
- `web/src/pages/WorkflowsPage.tsx`：工作流可视化编辑、节点配置、dry-run/validate/run/SSE。

### SDK 与 Demo

- `sdks/java/tikeo/src/main/java/net/tikeo/worker/client/GrpcTikeoWorkerClient.java`：Java Worker Tunnel 客户端。
- `sdks/java/tikeo/src/main/java/net/tikeo/processor/*`：Java processor API 与注解。
- `sdks/java/tikeo/src/main/java/net/tikeo/management/*`：Java 管理客户端。
- `sdks/java/tikeo-spring/src/main/java/net/tikeo/spring/*`：Spring processor registry/adapter。
- `sdks/java/tikeo-spring-boot-starter/src/main/java/net/tikeo/boot/*`：Spring Boot starter 自动配置与生命周期。
- `examples/java/spring-boot3-worker-demo/*`：Java Spring Boot demo。
- `sdks/rust/tikeo/src/*`：Rust SDK、session、management、script/wasm 支持。
- `examples/rust/worker-demo/src/main.rs`：Rust worker demo。
- `sdks/go/tikeo/*`：Go SDK/proto 基础，未达到完整 parity。

### 脚本、WASM、安全执行

- `crates/tikeo-wasm/src/lib.rs`：Wasmtime executor、fuel/timeout/memory/no ambient FS/network 校验。
- `sdks/java/tikeo/src/main/java/net/tikeo/script/*`：ScriptRunnerKind、ScriptSandboxBackend、WasmScriptRunner、ContainerScriptRunner、LocalSubprocessScriptRunner。
- `web/src/pages/ScriptsPage.tsx`：脚本 CRUD、版本、diff、publish/rollback、审批/签名/策略元数据 UI。

### 管理平台

- `crates/tikeo-server/src/http/openapi.rs`：OpenAPI 汇总。
- `crates/tikeo-server/src/http/auth.rs`：登录、OIDC、API Token 等。
- `crates/tikeo-server/src/http/sdk_api_keys.rs`：SDK API Key 管理。
- `crates/tikeo-server/src/http/routes/scope.rs`：namespace/app/worker pool 管理。
- `crates/tikeo-server/src/http/routes/audit_logs.rs`：审计日志查询/导出。
- `crates/tikeo-server/src/http/routes/metrics.rs`：业务 SLO 与 Prometheus 指标刷新。
- `crates/tikeo-server/src/observability/tracing.rs`：OTLP tracing。
- `crates/tikeo-server/src/alert.rs`、`alert/email.rs`、`alert/retry.rs`、`http/routes/alerts.rs`：告警渠道、重试、事件与规则 API。
- `web/src/pages/WorkersPage.tsx`、`ScopesPage.tsx`、`PluginsPage.tsx`、`AlertsPage.tsx`、`AuditLogsPage.tsx`：平台治理页面。

### 部署与运维

- `Dockerfile`、`docker-compose.yml`：基础容器化。
- `deploy/compose/*`：Compose 部署。
- `deploy/systemd/*`：Systemd 部署。
- `deploy/bare-metal/*`：裸机部署辅助。
- `deploy/k8s/tikeo.yaml`、`deploy/k8s/README.md`：K8s baseline。
- `deploy/gitops/tikeo-manifest.example.yaml`、`deploy/k8s/crd/tikeo-manifest-crd.yaml`、`deploy/k8s/operator/*`、`deploy/terraform/provider/*`、`deploy/terraform/tikeo_gitops_manifest.tf`：GitOps/IaC manifest、CRD、K8s controller/operator 与 Terraform Provider。
- `observability/prometheus/*`、`observability/grafana/*`、`docs/operations/prometheus-grafana-runbook.md`：监控运维材料。

---

## 6. 最终判定

按当前代码实际功能，tikeo 已经具备“核心调度平台 + Java/Rust Worker SDK + Web 管理台 + 工作流 DAG/回放 + 脚本/wasm/动态语言治理 + Service Account/RBAC + 可观测性/审计”的可联调基础。

本轮 P1 缺口已清空，Terraform Provider 与 K8s CRD controller/operator 已从 P2 缺口中移除。仍未完全覆盖设计表全部 ✅ 的部分集中在 P2：非 Java SDK parity（Go/Python/Node 已明确后续）；这些应继续保留在路线图/Phase 清单中。
