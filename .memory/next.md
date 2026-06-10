# Latest completed slice

- 2026-06-10: Docs module migration, docs Docker publishing setup, SEO/search/llms readiness, source-backed user guides, and the pre-migration acceptance runbook follow-up are in place. The Docusaurus docs site module is `docs/`; shared media is under `assets/docs/`; docs Docker build/publish targets `yhyzgn/tikeo-docs`; and docs now include source-backed English/zh-CN runbooks for `scripts/management-trigger-e2e-smoke.sh` plus Nginx Ingress / Envoy Gateway / Traefik / Gateway API Kubernetes controller guidance. Follow-up prompt: `.prompt/163-docs-publish-verification-and-acceptance-followup.md`.

# Next Work

## Current priority direction

当前优先级仍是功能/模块测试验收阶段，不收缩、不臆造。已完成 docs scaffold、P0 内容深度、zh-CN 路由镜像、部署 runbook、SDK create+trigger 文档、Management API trigger e2e smoke、source-derived Management OpenAPI / Worker Tunnel protobuf reference、docs module migration、docs Docker publishing setup、SEO/search/llms readiness、用户指南深度、Management trigger smoke 贡献者 runbook，以及 Kubernetes controller-specific 文档。下一步应优先做 release/credential-gated 的真实发布验证或新发现的真实缺口，不要重复做已完成的 rename/runbook 工作。

## Immediate next slice

1. Trigger `Publish / Docker docs` on a current ref/tag and record the Docker Hub digest for `yhyzgn/tikeo-docs`. Existing Docker Hub secrets are likely already available because `Publish / Docker server` and `Publish / Docker web` have succeeded; the docs workflow itself just has not published an image from the new `docs/` module yet.
2. If no release tag should be created yet, use manual workflow dispatch with a non-release image tag such as `main-<short-sha>` and `ref=main`; otherwise create/push the intended release tag. Continue acceptance on newly discovered runtime gaps only.
3. 迁移工具（PowerJob/XXL-JOB）仍维持最低优先级 backlog，核心服务体验稳定后再做。

## Current verified baseline

- Docs site module：默认 `/` 为英文站，`/zh-CN/` 为中文站；Docusaurus navbar/footer/sidebar/homepage/blog 均已本地化；`docs/docs/` 当前 P0 英文页面、user-guide 页面和 deployment runbooks 通过最小深度/section/source-backed 契约；`docs/i18n/zh-CN/docusaurus-plugin-content-docs/current/` 覆盖所有当前 P0、user-guide 和 runbook routes；共享 README/media assets 位于 `assets/docs/`。
- Docs publishing/search：`docs/Dockerfile` 使用 Bun builder + nginx runtime；CI docs Docker build uses `context: docs` / `file: docs/Dockerfile` / `push: false`; publish workflow targets `yhyzgn/tikeo-docs`; docs static entrypoints include `robots.txt`, `search-index.json`, `llms.txt`, `llms-full.txt`, and `static/img/tikeo-og.png`, and a local `/search/` page backed by `search-index.json`.
- Source-derived reference：`docs/docs/reference/management-openapi.md` / zh-CN mirror document `/api-docs/openapi.json`, `/api/v1/jobs`, `/api/v1/jobs/{job}:trigger`, `/api/v1/instances/{instance}`, `/api/v1/instances/{instance}/logs`, `CreateJobRequest`, `TriggerJobRequest`, `ApiResponse`, and `x-tikeo-api-key`; `docs/docs/reference/worker-tunnel-protobuf.md` / zh-CN mirror document `WorkerTunnelService`, `OpenTunnel`, `SubscribeTaskLogs`, `RegisterWorker`, `Heartbeat`, `WorkerRegistered`, `DispatchTask`, `TaskLog`, `TaskResult`, `TaskCheckpoint`, `assignment_token`, and `processor_name`.
- Acceptance runbooks：`docs/docs/deployment/management-trigger-smoke-runbook.md` / zh-CN mirror cover the actual smoke script, evidence files, case IDs and failure triage; `docs/docs/deployment/kubernetes-controller-runbook.md` / zh-CN mirror cover Nginx Ingress, Envoy Gateway, Traefik, Gateway API, TLS/mTLS matrix, NetworkPolicy, and outbound-only Worker Tunnel boundary.
- Main CI baseline：main CI contains `workflow-policy` repository contract tests, `Docs site` job, cross-language worker parity smoke, management-trigger e2e smoke artifact upload, and split Docker build validations for server/web/docs.
- Source-size cleanup：`scripts/check-source-size.py` 已覆盖普通 `.rs` / `.ts` / `.tsx` 源码并排除 `.git`、`.dev`、`target`、`node_modules`、`dist`、`coverage` 等生成/依赖目录；当前全仓库审计通过，且已接入 main CI `workflow-policy` 快速门禁。

## Standing constraints

- Functional/module testing acceptance phase: do not shrink scope; if anything missing/incomplete/untested/hallucinated is found, fill it production-grade or record a real blocker. Keep durable context fresh and source-backed.
- Worker 重要可见性状态不得只在内存。
- 禁止约定命名匹配；必须使用结构化字段、labels 或 structuredCapabilities。
- 中文 i18n 必须完整中文，英文 i18n 必须英文，不要中英混合 label。
- Go/Rust/Java/Python/Node SDK demo 能力广告必须真实；不可执行 sandbox 只能 fail-closed，不能作为 capability 暴露。
- 新 schema 变更必须进入显式 SeaORM migration；不得在 `connect_and_migrate` 后挂未记录的兼容补丁。
- Helm chart 不能部署业务 Worker 或创建业务 Worker 入站 Service；Worker 只能主动出站连接 Tikeo Worker Tunnel。
- 源文件 <=1500 行；`mod.rs` / `lib.rs` 等入口文件只做声明和 re-export；后续源码变更必须保持审计通过。
- Web/frontend package management and command execution must use `bun` / `bunx` unless explicitly overridden.
