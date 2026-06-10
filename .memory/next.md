# Latest completed slice

- 2026-06-10: Docs module migration and publishing/search/user-guide completion are in place. The Docusaurus docs site module is now `docs/`, old shared docs media moved to `assets/docs/`, CI validates docs build plus docs Docker build, `publish-docker-docs.yml` publishes `yhyzgn/tikeo-docs`, and docs now include SEO/search/robots/OpenGraph/llms entrypoints plus source-backed English/zh-CN user guides for Dashboard, Jobs, Instances, Workers, Workflows, Scripts, Audit, and Settings. Follow-up prompt: `.prompt/162-docs-module-docker-and-acceptance-followup.md`.

# Next Work

## Current priority direction

当前优先级：功能/模块测试验收阶段继续保持不收缩原则。独立 Docusaurus docs 站点已经完成 scaffold、P0 内容深度、zh-CN 路由镜像、部署 runbook、SDK create+trigger 文档、Management API trigger e2e smoke、source-derived Management OpenAPI / Worker Tunnel protobuf reference、docs module migration、docs Docker publishing、SEO/search/llms readiness 和用户指南深度。下一步应优先做真实验收证据扩展，而不是重复改名：docs Docker push workflow dry-run/发布验证、Management API trigger smoke 贡献者 runbook、Kubernetes controller-specific production docs，或继续补功能模块验收中发现的真实缺口。

## Immediate next slice

1. Verify docs image publishing on a release/manual workflow run when credentials and tag are available; record the Docker Hub digest for `yhyzgn/tikeo-docs`.
2. Add contributor runbook for `scripts/management-trigger-e2e-smoke.sh`:
   - prerequisites
   - `TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh`
   - evidence directory and failure triage
3. Kubernetes 后续可继续补真实控制器专项文档：Nginx/Envoy/Traefik/Gateway API controller 的实际生产 values、证书模式和 smoke runbook。
4. 迁移工具（PowerJob/XXL-JOB）仍维持最低优先级 backlog，核心服务体验稳定后再做。

## Current verified baseline

- Docs site module：默认 `/` 为英文站，`/zh-CN/` 为中文站；Docusaurus navbar/footer/sidebar/homepage/blog 均已本地化；`docs/docs/` 当前 P0 英文页面和 user-guide 页面通过最小深度/section/source-backed 契约；`docs/i18n/zh-CN/docusaurus-plugin-content-docs/current/` 覆盖所有当前 P0 与 user-guide routes，并通过 zh-CN 内容深度契约；共享 README/media assets 位于 `assets/docs/`。
- Docs publishing/search：`docs/Dockerfile` 使用 Bun builder + nginx runtime；CI docs Docker build uses `context: docs` / `file: docs/Dockerfile` / `push: false`; publish workflow targets `yhyzgn/tikeo-docs`; docs static entrypoints include `robots.txt`, `search-index.json`, `llms.txt`, `llms-full.txt`, and `static/img/tikeo-og.png`, and a local `/search/` page backed by `search-index.json`.
- Source-derived reference：`docs/docs/reference/management-openapi.md` / zh-CN mirror document `/api-docs/openapi.json`, `/api/v1/jobs`, `/api/v1/jobs/{job}:trigger`, `/api/v1/instances/{instance}`, `/api/v1/instances/{instance}/logs`, `CreateJobRequest`, `TriggerJobRequest`, `ApiResponse`, and `x-tikeo-api-key`; `docs/docs/reference/worker-tunnel-protobuf.md` / zh-CN mirror document `WorkerTunnelService`, `OpenTunnel`, `SubscribeTaskLogs`, `RegisterWorker`, `Heartbeat`, `WorkerRegistered`, `DispatchTask`, `TaskLog`, `TaskResult`, `TaskCheckpoint`, `assignment_token`, and `processor_name`.
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
