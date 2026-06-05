# Next Work

## Current priority direction

当前优先级：把已经落地的 cross-language Worker parity harness 纳入 CI/发布节奏，并继续清理生产化风险（迁移版本化、部署 Helm/外部 DB/TLS、Web 浏览器级截图/视频证据）。最新 harness 证据：`.dev/reports/cross-language-workers-20260605T032108Z-202626/`。

## Immediate next slice

1. 将 `deploy/smoke/cross-language-worker-parity-smoke.sh` 接入 GitHub Actions nightly 或 manual workflow，上传 `.dev/reports/cross-language-workers-*` artifacts。
2. 评估是否把 smoke 中 server/web/worker 端口参数化为 CI 并发安全矩阵。
3. 继续推进生产迁移策略：把当前兼容型 migration 收敛为显式版本化迁移与跨库回滚/前滚文档。
4. Web 端后续可用 Playwright 真实浏览器 screenshot/video 增强 Workers 分组页、dispatch queue 二级页和 API-Key 页面。
5. Python/Node SDK demo 仍是明确未来项；实现前不得在 examples README 中宣称 runnable。

## Current verified baseline

- Java demos：`examples/java/spring-boot2-worker-demo`、`spring-boot3-worker-demo`、`spring-boot4-worker-demo`。
- Go SDK/demo：默认 live；不默认广告不可执行脚本 runner；`go demo echo processed` 实例日志已由 harness 验证。
- Rust SDK/demo：默认 live；支持 success message；`rust demo echo processed` 实例日志已由 harness 验证。
- Worker visibility：`worker_sessions` 持久化 capabilities/structuredCapabilities/labels/master snapshot；server restart snapshot smoke 已通过。
- Web Worker：按 namespace/app 与 cluster/region 分组；dispatch queue 在 `/workers/dispatch-queue`；route smoke 已通过。

## Standing constraints

- Worker 重要可见性状态不得只在内存。
- 禁止约定命名匹配；必须使用结构化字段、labels 或 structuredCapabilities。
- 中文 i18n 必须完整中文，英文 i18n 必须英文，不要中英混合 label。
- Go/Rust/Java SDK demo 能力广告必须真实；不可执行 sandbox 只能 fail-closed，不能作为 capability 暴露。
- 源文件 <=1500 行；`mod.rs` / `lib.rs` 等入口文件只做声明和 re-export。
