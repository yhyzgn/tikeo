# Latest completed slice

- 2026-06-10: Management API trigger e2e smoke is now repeatable and CI-wired: `scripts/management-trigger-e2e-smoke.sh` starts an isolated SQLite server, creates app-scoped SDK API-Key credentials, registers a Node.js demo worker over outbound Worker Tunnel, creates/triggers a job through the Node.js SDK, and asserts instance/result/log evidence. Follow-up prompt: `.prompt/160-api-reference-and-docs-publishing-followup.md`.

# Next Work

## Current priority direction

当前优先级：独立 Docusaurus docs 站点已经完成 Phase A scaffold、Phase B 当前 P0 内容深度、Phase C 当前 P0 zh-CN 路由镜像，修复了独立站根路径与 GitHub Pages 子路径两种语言切换到中文 404 的 baseUrl 问题，并完成 Docusaurus 导航/侧边栏/页脚/首页/发布日志的中英文隔离；部署文档已补到复制即用级别，Compose 页面已直接写出完整 docker-compose*.yml；docs 验证已经接入主 CI；SDK create+trigger 文档已覆盖五种语言并由契约测试保护；Management API create+trigger 已有真实 server+worker+SDK e2e smoke 并接入 cross-language CI job。下一步应优先补用户指南/API reference 深度，并选择最终 docs hosting/search/SEO 发布策略。

## Immediate next slice

1. Add source-derived OpenAPI/protobuf reference pages and link SDK helpers to exact management endpoints / Worker Tunnel messages.
2. Add docs search/publish readiness once hosting target is selected: canonical URL, robots policy, OpenGraph image, local search or DocSearch plan, and generated/maintained `llms.txt` strategy.
3. Expand next docs depth from verified artifacts: SDK overview/cross-language parity, user-guide pages for Dashboard/Jobs/Instances/Workers/Workflows/Scripts/Audit/Settings, and generated OpenAPI/protobuf references.
5. Kubernetes 后续可继续补真实控制器专项文档：Nginx/Envoy/Traefik/Gateway API controller 的实际生产 values、证书模式和 smoke runbook。
6. 宣传录屏本地证据已完成：最终推荐版为 `.dev/reports/promo-cinematic-showcase-20260608T050247Z-231970/tikeo-cinematic-promo-hq-sentence-subs.mp4`；同目录保留逐句/短语级 `subtitles.en.srt`、`subtitles.zh-CN.srt`、`subtitles.bilingual.srt` 用于平台单独上传 CC 字幕。
7. 迁移工具（PowerJob/XXL-JOB）仍维持最低优先级 backlog，核心服务体验稳定后再做。

## Current verified baseline

- Docs site P0 content/localization/deployment：默认 `/` 为英文站，`/zh-CN/` 为中文站；Docusaurus navbar/footer/sidebar/homepage/blog 均已本地化；`website/docs/` 当前 P0 英文页面通过最小深度/section 契约；`website/i18n/zh-CN/docusaurus-plugin-content-docs/current/` 覆盖所有当前 P0 route，并通过 zh-CN 内容深度契约；SDK docs 覆盖 Rust、Go、Java Spring Boot、Python、Node.js，并已包含 source-backed Management API create+trigger examples、`x-tikeo-api-key` / `TIKEO_MANAGEMENT_API_KEY`、默认 `triggerType=api` + `executionMode=single` 与显式 broadcast selector helper 文档；部署 docs 覆盖 single binary/systemd、Compose SQLite/PostgreSQL/MySQL（含完整 docker-compose*.yml）、Helm dev/prod/TLS/ops 和配置参数。
- Docs/e2e verification：`python3 .github/tests/docs_site_contract_test.py`、`python3 .github/tests/workflow_contract_test.py`、`python3 .github/tests/management_smoke_contract_test.py`、`python3 scripts/check-source-size.py`、`cd website && bun install --frozen-lockfile && bun run docs:typecheck && bun run docs:build` 均通过；默认 root `/` serve smoke 与可选 `/tikeo/` subpath serve smoke 都验证 zh-CN route 非 404；主 CI 已新增 `Docs site` job 执行 docs contract、Bun frozen install、typecheck 与 build；`workflow-policy` 运行 repository contract tests；cross-language smoke job 运行 `scripts/management-trigger-e2e-smoke.sh` 并上传 `management-trigger-e2e-smoke` artifact；`website/bun.lock` 使用公开 npm registry tarball URL。
- Source-size cleanup：`scripts/check-source-size.py` 已覆盖普通 `.rs` / `.ts` / `.tsx` 源码并排除 `.git`、`.dev`、`target`、`node_modules`、`dist`、`coverage` 等生成/依赖目录；当前全仓库审计通过，且已接入 main CI `workflow-policy` 快速门禁。
- Main CI 基线：run `27129836559` succeeded for source commit `e98f6fd7395f1c104050ce8037db79ab5447aed6`，覆盖 Server/Web/Java/Rust/Go/Python/Node SDK+demo、deploy tooling、cross-language worker smoke 与 Docker build validation。
- Coverage 基线：run `27129836631` succeeded for source commit `e98f6fd7395f1c104050ce8037db79ab5447aed6`；Rust/Web/Java/Go/Python/Node coverage jobs 均通过并上传。
- Helm production + ops baseline：`deploy/helm/tikeo` 已支持外部数据库 Secret、SQLite PVC 条件化、TLS/mTLS Secret mounts、PDB、NetworkPolicy、ServiceMonitor、Gateway API `GRPCRoute`、`values.schema.json`、worker identity 文档和 rollback runbook。
- Browser promo artifact：最终推荐本地 MP4 位于 `.dev/reports/promo-cinematic-showcase-20260608T050247Z-231970/tikeo-cinematic-promo-hq-sentence-subs.mp4`，`ffprobe` 验证 496.520s、1920x1080、英文默认音轨、中文第二音轨、英/中文字幕逐句软字幕轨、无烧录字幕、CRF 16 高画质封装。
## 0.2.0 release follow-up

- `v0.2.0` formal release is published: https://github.com/yhyzgn/tikeo/releases/tag/v0.2.0
- Tag-triggered Rust/Python/Node/Java/Go SDK publishing, Docker server image publishing, GitHub assets, and follow-up Docker web image publishing have completed successfully.
- Next product/docs slice remains: final docs hosting configuration, docs search/SEO/OG readiness, deeper source-backed API/protobuf references, and SDK create+trigger examples.

## Standing constraints

- Worker 重要可见性状态不得只在内存。
- 禁止约定命名匹配；必须使用结构化字段、labels 或 structuredCapabilities。
- 中文 i18n 必须完整中文，英文 i18n 必须英文，不要中英混合 label。
- Go/Rust/Java/Python/Node SDK demo 能力广告必须真实；不可执行 sandbox 只能 fail-closed，不能作为 capability 暴露。
- 新 schema 变更必须进入显式 SeaORM migration；不得在 `connect_and_migrate` 后挂未记录的兼容补丁。
- Helm chart 不能部署业务 Worker 或创建业务 Worker 入站 Service；Worker 只能主动出站连接 Tikeo Worker Tunnel。
- 源文件 <=1500 行；`mod.rs` / `lib.rs` 等入口文件只做声明和 re-export；后续源码变更必须保持审计通过。

## SDK management trigger parity baseline

- Java/Rust/Go/Python/Node.js SDKs now expose app-scoped Management API create+trigger helpers.
- Java now also has explicit broadcast selector helper parity: `BroadcastSelectorRequest` + `TriggerJobRequest.broadcastApi(...)`.
- Rust/Go/Python/Node.js demos trigger created jobs under `TIKEO_MANAGEMENT_CREATE_EXAMPLES=1`; Java Boot2/3/4 demos expose documented controller endpoints for create+trigger examples.
- Website SDK docs now source-link the helper names and auth/default/broadcast semantics in English and zh-CN.
- Management create+trigger parity is now promoted from per-SDK/mock/demo tests into a real full server+worker e2e smoke: `scripts/management-trigger-e2e-smoke.sh`.
