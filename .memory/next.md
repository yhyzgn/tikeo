# Next Work

## Current priority direction

当前优先级：生产化部署 Helm baseline 已完成本地验证，README/宣传素材/coverage/CI 也已完成远端验证。下一步应先检查本轮 Helm hardening 提交推送后的远端 CI；若 CI 通过，继续推进部署成熟度的后续小切片，或按用户批准启动独立 docs 站点搭建。

## Immediate next slice

1. 推送本轮 Helm hardening 后先查看最新 GitHub Actions：如果失败，按 job 分组日志修复，不要恢复旧的碎片化 job 命名。
2. 部署成熟度后续建议：补可选 PodDisruptionBudget、NetworkPolicy、ServiceMonitor/Prometheus scrape、Gateway API h2 Worker Tunnel 示例与 `values.schema.json`。保持 Worker 出站连接模型，不要在 chart 中给业务 Worker 暴露入站 Service。
3. 先处理源码行数历史债务或给 CI 加明确豁免边界：当前审计发现 `dispatcher.rs`、`repository.rs`、`workflow.rs`、`web/src/i18n/messages.ts`、`web/src/api/client.ts` 等历史文件超过 1500 行，不能继续宣称全仓库已满足该规则。
4. 文档站搭建方案已输出到 `design/docs-site-build-plan.md`；若用户批准实施，下一步创建独立 `website/` Docusaurus 3 站点，先完成导航骨架、英文 P0 页面、中文 i18n 路径和 docs build 验证，不要在未实现前宣称部署完成。
5. 宣传录屏本地证据已完成：最终推荐版为 `.dev/reports/promo-cinematic-showcase-20260608T050247Z-231970/tikeo-cinematic-promo-hq-sentence-subs.mp4`；同目录保留逐句/短语级 `subtitles.en.srt`、`subtitles.zh-CN.srt`、`subtitles.bilingual.srt` 用于 YouTube/X/Reddit/Bilibili 等平台单独上传 CC 字幕。若要公开分发，下一步可做剪辑压缩、封面海报、上传/CDN 或 CI artifact 化。
6. 迁移工具（PowerJob/XXL-JOB）仍维持最低优先级 backlog，核心服务体验稳定后再做。

## Current verified baseline

- Main CI 基线：run `27128044956` succeeded for source commit `c90b44177a692946ad4cd000f16e6653ddc508e9`，覆盖 Server/Web/Java/Rust/Go/Python/Node SDK+demo、deploy tooling、cross-language worker smoke 与 Docker build validation。
- Coverage 基线：run `27128044845` succeeded for source commit `c90b44177a692946ad4cd000f16e6653ddc508e9`；Rust/Web/Java/Go/Python/Node coverage jobs 均通过并上传。
- Helm production baseline：`deploy/helm/tikeo` 已支持外部 PostgreSQL/MySQL/CockroachDB URL Secret、SQLite PVC 条件化、HTTP/Worker Tunnel TLS/mTLS Secret mounts、transport security config 渲染、server/web ingress、probe/resource/securityContext 参数、worker identity 文档和 rollback runbook；本地 `helm lint` 与三套 `helm template` 场景通过。
- Java demos：`examples/java/spring-boot2-worker-demo`、`spring-boot3-worker-demo`、`spring-boot4-worker-demo`。
- Go SDK/demo：默认 live；不默认广告不可执行脚本 runner；`go demo echo processed` 实例日志已由 harness 验证。
- Rust SDK/demo：默认 live；支持 success message；`rust demo echo processed` 实例日志已由 harness 验证。
- Python SDK/demo 与 Node.js SDK/demo：仓库中已有真实目录、测试与 CI/coverage gate；不要再把它们作为“目录缺失时未来项”描述。后续仍可继续增强 ergonomics/live parity，但 README/examples 不应宣称未实现。
- Worker visibility：`worker_sessions` 持久化 capabilities/structuredCapabilities/labels/master snapshot；server restart snapshot smoke 已通过。
- Web Worker：按 namespace/app 与 cluster/region 分组；dispatch queue 在 `/workers/dispatch-queue`；route smoke 已通过。
- GitHub discovery polish：README 首屏动图、短卖点、Star History、开源治理文件、issue/PR templates 和 GitHub topics/description 已完成。
- Docs site plan：`design/docs-site-build-plan.md` 已完成，明确参考 Hermes-style Docusaurus IA、双语站点、LLM exports、P0 页面和未来验证命令；尚未搭建或部署实际站点。
- README badge/runtime polish：CI 改用稳定静态 Shields badge；SDK runtime requirement badges 已放在全部 SDK 版本徽章之前；overall Codecov badge 已接入并远端返回真实百分比。
- Browser promo artifact：最终推荐本地 MP4 位于 `.dev/reports/promo-cinematic-showcase-20260608T050247Z-231970/tikeo-cinematic-promo-hq-sentence-subs.mp4`，`ffprobe` 验证 496.520s、1920x1080、英文默认音轨、中文第二音轨、英/中文字幕逐句软字幕轨、无烧录字幕、CRF 16 高画质封装；字幕从 12 条章节级长字幕优化为英文 72 条、中文 57 条；抽帧确认 Worker 页面硬编码中文漏点已改为英文。
- Storage migration：SQLite legacy schema compatibility 已迁入显式 SeaORM migration `sqlite_compat`，由 `seaql_migrations` 持久记录；验证命令见 progress/session-log。

## Standing constraints

- Worker 重要可见性状态不得只在内存。
- 禁止约定命名匹配；必须使用结构化字段、labels 或 structuredCapabilities。
- 中文 i18n 必须完整中文，英文 i18n 必须英文，不要中英混合 label。
- Go/Rust/Java/Python/Node SDK demo 能力广告必须真实；不可执行 sandbox 只能 fail-closed，不能作为 capability 暴露。
- 新 schema 变更必须进入显式 SeaORM migration；不得在 `connect_and_migrate` 后挂未记录的兼容补丁。
- 源文件 <=1500 行；`mod.rs` / `lib.rs` 等入口文件只做声明和 re-export。当前存在历史超限文件，后续不能宣称已全仓库满足，需优先拆分或建立清晰豁免规则。
