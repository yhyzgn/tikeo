# Next Work

## Current priority direction

当前优先级：数据库迁移版本化硬化已进入收尾验证阶段；cross-language Worker parity harness 已纳入主 CI 并上传 artifact，Docker validation 已拆分 server/web 且 Node runtime policy 已全绿。2026-06-08 已补强本地 Playwright 宣传录屏证据，并产出富数据、全英文站点 UI、动态滚动/聚焦、英文默认/中文第二音轨、英中逐句软字幕、不烧录字幕、1080p 高画质本地 MP4。下一步继续推进生产化风险：部署 Helm/外部 DB/TLS/Secret 模板硬化。

## Immediate next slice

1. 下次接手先查看最新 CI：本轮 CI 分组提交按用户指示未等待远端结果；如果失败，优先按 job 分组日志修复，但不要恢复旧的碎片化 job 命名。
2. 先处理源码行数历史债务或给 CI 加明确豁免边界：当前审计发现 `dispatcher.rs`、`repository.rs`、`workflow.rs`、`web/src/i18n/messages.ts`、`web/src/api/client.ts` 等历史文件超过 1500 行，不能继续宣称全仓库已满足该规则。
3. 继续部署生产化专项：Helm values、外部 PostgreSQL/MySQL/CockroachDB 连接、TLS/mTLS secret、readiness/liveness、worker identity env 和回滚文档。
4. 宣传录屏本地证据已完成：最终推荐版为 `.dev/reports/promo-cinematic-showcase-20260608T050247Z-231970/tikeo-cinematic-promo-hq-sentence-subs.mp4`；同目录保留逐句/短语级 `subtitles.en.srt`、`subtitles.zh-CN.srt`、`subtitles.bilingual.srt` 用于 YouTube/X/Reddit/Bilibili 等平台单独上传 CC 字幕。若要公开分发，下一步可做剪辑压缩、封面海报、上传/CDN 或 CI artifact 化。
5. 开源传播首屏优化已完成：README/中文 README 已加入 1.58MB 控制台 tour GIF、首屏卖点、Star History、支持提示；已补齐 CONTRIBUTING/SECURITY/CODE_OF_CONDUCT/CHANGELOG/ROADMAP 与 GitHub issue/PR templates；GitHub description/topics 已同步。
6. 文档站搭建方案已输出到 `design/docs-site-build-plan.md`；若用户批准实施，下一步创建独立 `website/` Docusaurus 3 站点，先完成导航骨架、英文 P0 页面、中文 i18n 路径和 docs build 验证，不要在未实现前宣称部署完成。
7. Coverage 已扩展到全项目主要 surface：`.github/workflows/coverage.yml` 使用 direct `codecov-cli` 上传 Rust workspace、Web、Java SDK、Go SDK、Python SDK/demo、Node.js SDK 的覆盖率报告；本地报告生成已通过，远端全量 Coverage workflow 需在本轮 push 后确认。此前 Rust-only run `27121393205` 已证明 `CODECOV_TOKEN` 可用并返回 Rust flag 84%。
8. 保留 Python/Node SDK demo 为明确未来项；实现前不得在 examples README 中宣称 runnable。
9. 迁移工具（PowerJob/XXL-JOB）仍维持最低优先级 backlog，核心服务体验稳定后再做。

## Current verified baseline

- 最新 CI 基线：commit `5027e82` / run `27004107956` 全绿；`gh run view` 未出现 Node.js 20 warning 文案。
- Java demos：`examples/java/spring-boot2-worker-demo`、`spring-boot3-worker-demo`、`spring-boot4-worker-demo`。
- Go SDK/demo：默认 live；不默认广告不可执行脚本 runner；`go demo echo processed` 实例日志已由 harness 验证。
- Rust SDK/demo：默认 live；支持 success message；`rust demo echo processed` 实例日志已由 harness 验证。
- Worker visibility：`worker_sessions` 持久化 capabilities/structuredCapabilities/labels/master snapshot；server restart snapshot smoke 已通过。
- Web Worker：按 namespace/app 与 cluster/region 分组；dispatch queue 在 `/workers/dispatch-queue`；route smoke 已通过。
- GitHub discovery polish：README 首屏动图、短卖点、Star History、开源治理文件、issue/PR templates 和 GitHub topics/description 已完成。
- Docs site plan：`design/docs-site-build-plan.md` 已完成，明确参考 Hermes-style Docusaurus IA、双语站点、LLM exports、P0 页面和未来验证命令；尚未搭建或部署实际站点。
- README badge/runtime polish：CI 改用稳定静态 Shields badge；SDK runtime requirement badges 已放在全部 SDK 版本徽章之前；Rust Codecov coverage badge 已接入并返回 84%。
- Browser promo artifact：最终推荐本地 MP4 位于 `.dev/reports/promo-cinematic-showcase-20260608T050247Z-231970/tikeo-cinematic-promo-hq-sentence-subs.mp4`，`ffprobe` 验证 496.520s、1920x1080、英文默认音轨、中文第二音轨、英/中文字幕逐句软字幕轨、无烧录字幕、CRF 16 高画质封装；字幕从 12 条章节级长字幕优化为英文 72 条、中文 57 条；抽帧确认 Worker 页面硬编码中文漏点已改为英文。
- Storage migration：SQLite legacy schema compatibility 已迁入显式 SeaORM migration `sqlite_compat`，由 `seaql_migrations` 持久记录；本轮验证命令见 progress/session-log。

## Standing constraints

- Worker 重要可见性状态不得只在内存。
- 禁止约定命名匹配；必须使用结构化字段、labels 或 structuredCapabilities。
- 中文 i18n 必须完整中文，英文 i18n 必须英文，不要中英混合 label。
- Go/Rust/Java SDK demo 能力广告必须真实；不可执行 sandbox 只能 fail-closed，不能作为 capability 暴露。
- 新 schema 变更必须进入显式 SeaORM migration；不得在 `connect_and_migrate` 后挂未记录的兼容补丁。
- 源文件 <=1500 行；`mod.rs` / `lib.rs` 等入口文件只做声明和 re-export。当前存在历史超限文件，后续不能宣称已全仓库满足，需优先拆分或建立清晰豁免规则。
