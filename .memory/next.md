# Next Work

## Current priority direction

当前优先级：数据库迁移版本化硬化已进入收尾验证阶段；cross-language Worker parity harness 已纳入主 CI 并上传 artifact，Docker validation 已拆分 server/web 且 Node runtime policy 已全绿。下一步继续推进生产化风险：部署 Helm/外部 DB/TLS/Secret 模板硬化，以及 Web 真实浏览器 screenshot/video 证据增强。

## Immediate next slice

1. 完成本轮数据库迁移版本化提交后，确认 GitHub CI 全绿且无 Node.js 20 warning 文案。
2. 先处理源码行数历史债务或给 CI 加明确豁免边界：当前审计发现 `dispatcher.rs`、`repository.rs`、`workflow.rs`、`web/src/i18n/messages.ts`、`web/src/api/client.ts` 等历史文件超过 1500 行，不能继续宣称全仓库已满足该规则。
3. 继续部署生产化专项：Helm values、外部 PostgreSQL/MySQL/CockroachDB 连接、TLS/mTLS secret、readiness/liveness、worker identity env 和回滚文档。
4. Web 浏览器级验收增强：用 Playwright 对 Workers 分组页、dispatch queue 二级页、API-Key 页面输出 screenshot/video artifact。
5. 保留 Python/Node SDK demo 为明确未来项；实现前不得在 examples README 中宣称 runnable。
6. 迁移工具（PowerJob/XXL-JOB）仍维持最低优先级 backlog，核心服务体验稳定后再做。

## Current verified baseline

- 最新 CI 基线：commit `5027e82` / run `27004107956` 全绿；`gh run view` 未出现 Node.js 20 warning 文案。
- Java demos：`examples/java/spring-boot2-worker-demo`、`spring-boot3-worker-demo`、`spring-boot4-worker-demo`。
- Go SDK/demo：默认 live；不默认广告不可执行脚本 runner；`go demo echo processed` 实例日志已由 harness 验证。
- Rust SDK/demo：默认 live；支持 success message；`rust demo echo processed` 实例日志已由 harness 验证。
- Worker visibility：`worker_sessions` 持久化 capabilities/structuredCapabilities/labels/master snapshot；server restart snapshot smoke 已通过。
- Web Worker：按 namespace/app 与 cluster/region 分组；dispatch queue 在 `/workers/dispatch-queue`；route smoke 已通过。
- Storage migration：SQLite legacy schema compatibility 已迁入显式 SeaORM migration `sqlite_compat`，由 `seaql_migrations` 持久记录；本轮验证命令见 progress/session-log。

## Standing constraints

- Worker 重要可见性状态不得只在内存。
- 禁止约定命名匹配；必须使用结构化字段、labels 或 structuredCapabilities。
- 中文 i18n 必须完整中文，英文 i18n 必须英文，不要中英混合 label。
- Go/Rust/Java SDK demo 能力广告必须真实；不可执行 sandbox 只能 fail-closed，不能作为 capability 暴露。
- 新 schema 变更必须进入显式 SeaORM migration；不得在 `connect_and_migrate` 后挂未记录的兼容补丁。
- 源文件 <=1500 行；`mod.rs` / `lib.rs` 等入口文件只做声明和 re-export。当前存在历史超限文件，后续不能宣称已全仓库满足，需优先拆分或建立清晰豁免规则。
