# Next Work

## Current priority direction

当前优先级：源码行数历史债务已清理并有本地审计脚本；本轮提交后先检查远端 CI/Coverage。如果远端继续全绿，下一步建议在“文档站实现”与“源码行数审计接入 CI”之间二选一；若用户继续运营/宣传方向，优先落地 docs 站点骨架。

## Immediate next slice

1. 推送本轮 source-size cleanup 后查看最新 GitHub Actions CI/Coverage；若失败，按 job 日志修复，不要回退源码拆分边界。
2. 将 `python3 scripts/check-source-size.py` 接入 CI workflow / workflow contract（可作为 workflow policy 或 Server/Web 前的快速门禁），避免再次产生 >1500 行源码。
3. 文档站搭建方案已输出到 `design/docs-site-build-plan.md`；若用户批准实施，下一步创建独立 `website/` Docusaurus 3 站点，先完成导航骨架、英文 P0 页面、中文 i18n 路径和 docs build 验证，不要在未实现前宣称部署完成。
4. Kubernetes 后续可继续补真实控制器专项文档：Nginx/Envoy/Traefik/Gateway API controller 的实际生产 values、证书模式和 smoke runbook。
5. 宣传录屏本地证据已完成：最终推荐版为 `.dev/reports/promo-cinematic-showcase-20260608T050247Z-231970/tikeo-cinematic-promo-hq-sentence-subs.mp4`；同目录保留逐句/短语级 `subtitles.en.srt`、`subtitles.zh-CN.srt`、`subtitles.bilingual.srt` 用于平台单独上传 CC 字幕。
6. 迁移工具（PowerJob/XXL-JOB）仍维持最低优先级 backlog，核心服务体验稳定后再做。

## Current verified baseline

- Source-size cleanup：`scripts/check-source-size.py` 已覆盖普通 `.rs` / `.ts` / `.tsx` 源码并排除 `.git`、`.dev`、`target`、`node_modules`、`dist`、`coverage` 等生成/依赖目录；当前全仓库审计通过。
- 拆分边界：`dispatcher.rs` -> `dispatcher/processors.rs` + 分片测试；`registry.rs` -> `registry/registry_tests.rs`；`repository.rs` -> `repository/tests.rs` + 分片测试；`workflow.rs` -> `workflow/runtime.rs`；`migration/mod.rs` -> `migration/rbac_role_management.rs`；HTTP `part_03.rs` -> `part_03_a.rs`/`part_03_b.rs`；Web workflow/worker API -> `web/src/api/workflow.ts` 并从 `client.ts` re-export。
- Local verification for cleanup: source-size audit, git diff check, Rust fmt/clippy/test/build, Web lint/typecheck/test/build, and healthz smoke all passed locally before commit.
- Main CI 基线：run `27129836559` succeeded for source commit `e98f6fd7395f1c104050ce8037db79ab5447aed6`，覆盖 Server/Web/Java/Rust/Go/Python/Node SDK+demo、deploy tooling、cross-language worker smoke 与 Docker build validation。
- Coverage 基线：run `27129836631` succeeded for source commit `e98f6fd7395f1c104050ce8037db79ab5447aed6`；Rust/Web/Java/Go/Python/Node coverage jobs 均通过并上传。
- Helm production + ops baseline：`deploy/helm/tikeo` 已支持外部数据库 Secret、SQLite PVC 条件化、TLS/mTLS Secret mounts、PDB、NetworkPolicy、ServiceMonitor、Gateway API `GRPCRoute`、`values.schema.json`、worker identity 文档和 rollback runbook。
- Browser promo artifact：最终推荐本地 MP4 位于 `.dev/reports/promo-cinematic-showcase-20260608T050247Z-231970/tikeo-cinematic-promo-hq-sentence-subs.mp4`，`ffprobe` 验证 496.520s、1920x1080、英文默认音轨、中文第二音轨、英/中文字幕逐句软字幕轨、无烧录字幕、CRF 16 高画质封装。

## Standing constraints

- Worker 重要可见性状态不得只在内存。
- 禁止约定命名匹配；必须使用结构化字段、labels 或 structuredCapabilities。
- 中文 i18n 必须完整中文，英文 i18n 必须英文，不要中英混合 label。
- Go/Rust/Java/Python/Node SDK demo 能力广告必须真实；不可执行 sandbox 只能 fail-closed，不能作为 capability 暴露。
- 新 schema 变更必须进入显式 SeaORM migration；不得在 `connect_and_migrate` 后挂未记录的兼容补丁。
- Helm chart 不能部署业务 Worker 或创建业务 Worker 入站 Service；Worker 只能主动出站连接 Tikeo Worker Tunnel。
- 源文件 <=1500 行；`mod.rs` / `lib.rs` 等入口文件只做声明和 re-export；本轮已新增审计脚本，后续源码变更必须保持审计通过。
