# 风险与验证缺口

- UI 技术栈已固定为 React + TypeScript + Ant Design + Bun，但工程尚未初始化。
- 依赖安全审计命令尚未配置；bootstrap 阶段应考虑 cargo-deny/cargo-audit 与 Bun 依赖审计替代方案。

- 基础调度 tick loop、实例状态机和 Worker 任务分发尚未实现；当前只完成 Jobs 持久化与 Worker 注册/心跳 skeleton。
- MySQL migration 已通过 SeaORM feature 启用，但当前自动化验证只覆盖 SQLite in-memory 与 SQLite dev DB，尚未接入真实 MySQL 集成测试。
- OpenAPI JSON 路径为 `/api-docs/openapi.json`，不是早期提示词里的 `/openapi.json`。
- Worker Tunnel 当前只有注册/心跳 skeleton，尚未实现真实任务分发、取消、drain、证书轮换。
- Worker Tunnel 当前 smoke 只验证 9998 监听与单元测试，尚未加入真实 gRPC client 集成测试。
- Axum 0.8 不允许同一路径段内同时使用参数和字面量后缀；`/api/v1/jobs/{job}:trigger` 对外契约由内部 `/jobs/{job_action}` 路由承接并在 handler 中解析 `:trigger` 后缀。
- CRON / Fixed Rate tick loop 尚未实现；当前基础调度只覆盖 API 手动触发实例入库。

- Java SDK 当前只有 Spring Boot Starter 骨架、注解扫描和 Noop client，尚未生成/接入 Java gRPC Worker Tunnel 真实连接。
- Worker Tunnel proto RPC 已从 `Connect` 改为 `OpenTunnel`；外部 SDK 若已基于旧名生成代码需要同步更新。

- Web build 当前有 Vite 大 chunk 警告（Ant Design bundle），功能构建通过；后续可用动态 import / 路由拆包优化。
- Web 当前是管理端骨架，登录、RBAC、实例日志查看和实时事件流尚未实现。

- Docker/K8s 基础部署已验证；K8s 当前只有原始 YAML 与开发态 SQLite PVC，生产仍需要 Helm Chart、外部数据库、高可用、Ingress/Gateway、NetworkPolicy、PDB、ServiceMonitor。
- Web nginx 代理假设后端服务名为 `tikeo`；Compose 与当前 K8s YAML 已保持该名称，若 Helm/生产命名变化，需要模板化 upstream。

- 009 dispatch loop 当前是单节点 first-available worker 策略，尚未实现 capability/tag 匹配、租约过期剔除、任务 ack 超时、重试、幂等锁和多 server 协调。
- TaskResult 当前只落实例最终状态，尚未持久化 worker_id、错误信息、执行耗时和日志。

- 010 tikeo tick loop 的 schedule cursor 已持久化到 `schedule_cursors` 并由唯一键避免同一 fire_at 重复触发；后续仍需要补充多节点分布式锁/leader ownership 的生产部署压测。
- CRON / Fixed Rate 当前只创建 pending instance，尚未实现 misfire 策略、时区配置、暂停/恢复、最大并发、任务堆积保护。

- 011 日志当前按实例分页骨架返回全部结果，尚未实现游标分页、日志压缩/归档、实时 SSE/WebSocket 流和敏感信息脱敏策略。
- TaskLog 持久化当前信任 worker 上报的 sequence/level/message，后续需要大小限制、速率限制和租户隔离。



## 2026-05-19 — 012 auth 风险

- 当前认证是开发期基础，不是生产安全方案；默认 `tikeo_init/Tikeo@2026!` 和静态 bearer token 仅用于本地与早期集成。
- 尚未实现正式 RBAC、OIDC、API Token 生命周期、密码哈希、审计日志、CSRF/刷新 token 等生产能力。
- Web token 使用 `localStorage`，存在 XSS 后 token 泄露风险；正式安全阶段需要收敛为更安全的会话策略。


## 2026-05-19 — Docker host 网络风险

- 使用 `docker build --network host` 或运行时 host network 会掩盖容器 bridge DNS、端口映射、反向代理和多层网络路径问题。
- 后续部署相关验收必须优先覆盖 Docker bridge / Compose bridge；K8s、WAF、LB 层验证在后续 Helm / ingress 阶段继续补齐。


## 2026-05-19 — 013 broadcast / bridge container follow-up

- Broadcast target set is currently all online workers; 014 must add namespace/app/capability/label matching to prevent cross-app fan-out.
- Broadcast attempt dispatch has no lease timeout, retry, cancellation, or idempotency lock yet; multi-server coordination remains future work.
- Parent broadcast aggregation currently distinguishes all-success vs partial-failed after all children finish; richer per-attempt error metadata is not persisted yet.
- Docker bridge smoke validates Compose DNS/proxy locally, but WAF/LB/Ingress/Gateway/Service Mesh behavior still needs later deployment hardening tests.
- Web production build still emits a large chunk warning from Ant Design bundle; functionality passes, later route-level code splitting can optimize it.


## 2026-05-19 — dev bootstrap caveats

- Initialization credentials are intentionally documented for development; they must not be used as production credentials.
- `scripts/dev.sh` starts local long-running processes and writes `.dev/*` logs; `.dev/` is ignored by git.


## 2026-05-19 — UI / schema follow-up

- SQLite compatibility pass fixes current dev DB drift, but long-term production migrations should be split into explicit versioned migrations with downgrade/roll-forward policy.
- Web UI is visually modernized, but full UX depth for Worker, Security, Audit, Workflow, and Settings is still pending backend capability implementation.
- Vite build still reports a large bundle warning due to Ant Design; future routing/code-splitting should address this.


## 2026-05-20 — session abstraction follow-up

- 020 阶段已删除开发期 `tikeo-init-token` 静态 bearer backdoor；后续风险转为需要完善正式 RBAC / OIDC / API Token 生命周期管理。
- 当前 session TTL 固定在代码中，后续应进入配置文件并支持 Redis 分布式实现。
- moka 本地缓存不是权威状态；多节点部署前必须实现 Redis 或事件驱动的跨节点撤销同步。

## 2026-05-20 — soft relation risk

- 全库禁止数据库外键后，关系完整性必须由 repository/service 和测试保障；后续删除父记录时要显式处理子记录清理。
- SQLite 兼容层会重建历史外键表；生产数据库迁移也必须遵守无外键策略并单独验证。

## 021 后续风险

- RBAC 当前是最小 `resource/action` 模型；API Token 已有 create/list/revoke、细粒度 `resource:action` scope、TTL 策略、rotate 与 namespace/app/worker_pool binding 基础；tenant/app/worker-pool 后端管理 API 与 Web create/list/delete UI 已具备基础，删除策略采用非空拒绝而非隐式级联；租户隔离策略闭环和 OIDC 身份映射仍未实现。
- roles/permissions seed 当前主要覆盖内置角色；后续若开放角色管理 UI，需要补角色 CRUD、权限绑定审计和权限变更 session 失效。

## 2026-05-23 — Phase 3 closeout production gaps

- OIDC/SSO is fail-closed foundation only: generated one-time state, callback token exchange, discovery, and UserInfo retrieval exist, but external subject to local user/role/tenant mapping and opaque tikeo session issuance are not implemented.
- TLS/mTLS is config/status foundation only: TLS-enabled endpoints explicitly report `tls_pending_listener`; HTTP and Worker Tunnel still serve plaintext until real listener wiring lands.
- Script governance blocks unsafe releases and unverified approval/signature metadata, but full multi-level approval workflow, verified signatures/KMS, URL/File/Secret grants, and production release gates remain future work.
- Alerting has durable rules/events/recovery/summary, redacted channel readiness, production-guarded webhook/provider POST delivery, provider-specific Slack/DingTalk/Feishu/WeCom/PagerDuty JSON adapters, persisted delivery attempt history, local-loopback SMTP email delivery foundation, and bounded retry/DLQ processing plus ownership-gated background retry scheduling; production SMTP TLS/auth/secret handling and live external provider smoke remain future work.
- Observability has `/metrics`, metrics summary, Grafana template, dispatch queue pending-age and completed dispatch latency histograms, instance status/success-ratio, alert status, script-governance, and workflow/map-shard SLA Prometheus snapshots; live recording-rule validation and collector export smoke are still open.

## 2026-06-05 — Cross-language Worker parity automation gap

- Go/Rust/Java Worker parity and Worker session snapshot persistence have manual live evidence and CI success, but the cross-language, server-restart, persisted-filtering scenario still needs a committed executable harness.
- Risk: without automated restart persistence coverage, future changes could regress `/api/v1/workers` back to memory-only visibility or lose structuredCapabilities/labels/master snapshots after server restart.
- Risk: without explicit worker_pool filtering tests over persisted snapshots, convention-based matching could accidentally reappear.
- Required next mitigation: implement `.prompt/147-phase4-cross-language-worker-parity-and-persistence-hardening.md` and write evidence under `.dev/reports/cross-language-workers-<run-id>/`.

## 2026-06-05 — Cross-language Worker parity automation mitigated

- Previous risk that cross-language worker parity and server restart worker snapshot were only manually verified is mitigated by `deploy/smoke/cross-language-worker-parity-smoke.sh`.
- Remaining risk is CI coverage frequency: the harness starts server/web plus five demo workers and may be too heavy for every PR. Recommended mitigation is nightly/manual GitHub Actions with artifacts.
- Do not regress Go/Rust default capability advertising: unavailable script runners are fail-closed handlers only and must not appear in structured `scriptRunners`.

## 2026-06-10 — Docs CI registry risk mitigated

- `docs/bun.lock` previously pinned tarball URLs to a private Nexus npm proxy; local `bun install --frozen-lockfile` demonstrated the failure mode with 401 responses.
- The docs lockfile now uses public `https://registry.npmjs.org/` tarball URLs and `.github/tests/docs_site_contract_test.py` rejects private registry hosts.
- Remaining risk: if a future docs dependency refresh is run against a private registry, the contract test will fail until the lockfile is normalized back to a CI-accessible registry.

## 2026-06-10 — Docs publish and live controller verification gap

- Docs publish workflow and docs Docker image build are implemented and locally verified. `Publish / Docker server` and `Publish / Docker web` have already succeeded in GitHub Actions, so new Docker Hub credentials are not expected; the remaining gap is to trigger `Publish / Docker docs` on a current ref/tag and record the pushed `yhyzgn/tikeo-docs` digest.
- Kubernetes controller-specific runbooks are source-backed by committed Helm values/templates and include smoke commands for Nginx Ingress, Envoy Gateway, Traefik, and Gateway API; live controller acceptance still depends on an external cluster with the corresponding controllers/CRDs installed.

## 2026-06-10 — Docs manual depth and runnable quickstart risk mitigated

- Risk that the docs site was only a README rehash is mitigated by operator-grade contracts over critical English and zh-CN pages.
- Risk that quickstart runbooks could hallucinate bootstrap fields or provide unrunnable SDK scripts is mitigated by contracts checking `data.registrationOpen`, exported `TOKEN`, repository-root `tikeo-quickstart-trigger.ts`, real Node.js SDK exports, and Docusaurus build.
- Risk that docs containers behind local port mapping or reverse proxies redirect no-trailing-slash docs URLs to port 80 is mitigated by `absolute_redirect off` and `port_in_redirect off` in docs nginx config plus container route smoke.
- Remaining risk: live Docker Hub digest for `yhyzgn/tikeo-docs` is still workflow-trigger-gated; live Kubernetes controller smokes still require external clusters/controllers.

## 2026-06-11 — Alerting / Notification Center ambiguity risk

- Existing alerting has real provider adapters, delivery attempts, retry, and DLQ, but alert rules still embed `channels_json`. If future work simply adds another notification implementation without a boundary, Tikeo will duplicate credentials, delivery state, provider adapters, and UI concepts.
- Mitigation: treat Alerting as the abnormal-condition rule/event subsystem and Notification Center as the shared channel/template/policy/delivery subsystem. Preserve existing alert APIs during migration, but new job/workflow/alert touchpoints must use reusable notification channels and policies.
- Additional risk: job retry flows can spam final-failure notifications on every failed attempt. Mitigation: model `retry_scheduled` separately and emit terminal `failed` / `retry_exhausted` only after retries are exhausted.

## 2026-06-11 — Notification Center remaining hardening risks

- Alert rules still keep inline `channels_json` for compatibility. Reusable channel/policy migration for Alerting must be implemented before claiming alert delivery is fully unified under Notification Center.
- Workflow `notification` nodes still use raw `channel/target/template` config in the workflow editor/runtime shape; they must migrate to registered channels/templates before claiming workflow notification-node convergence.
- Generic delivery is now at-least-once rather than at-most-once: a crash after result row insertion but before old attempt consumption may duplicate delivery. Future mitigation should add lease/in-progress recovery and idempotency keys without reintroducing lost notifications.
- Live external provider smoke remains credential-gated. Local tests cover loopback/webhook/header/email mechanics and redaction, not Slack/DingTalk/Feishu/WeCom/PagerDuty live SaaS acceptance.

## 2026-06-11 — Notification provider schema/template hardening residual risks

- First-class reusable `notification_templates` CRUD/render is implemented and locally tested, but policies still soft-link by id/templateKey; deleting a template does not yet provide an impact preview or cascade guard for policies that reference it.
- Built-in provider payload shapes are source-backed and locally tested against loopback HTTP receivers, but live Slack/DingTalk/Feishu/WeCom/PagerDuty acceptance remains credential-gated.
- Channel test-send is still not implemented. Metadata must remain `supportsTestSend=false` until a real endpoint persists attempts and redacts results.
- Email exposes an HTML template shape for future compatibility, but the current SMTP adapter sends text/plain only; do not claim HTML/MIME email delivery until implemented and tested.

## 2026-06-13 — Job notification binding remaining risks

- Live external provider delivery remains credential-gated; local tests cover API validation, payload/trace materialization, redaction, and UI wiring but not real Slack/DingTalk/Feishu/WeCom/PagerDuty SaaS acceptance.
- Message trace log redaction is display-layer key-value redaction for common sensitive names; future work should centralize structured JSON log redaction if logs start storing rich secret-bearing objects.
- Alert rule automatic migration/dual-write to Notification Center policies is still open; do not claim alert delivery is fully unified until that compatibility migration lands.
- Generic notification delivery remains at-least-once; lease/idempotency hardening is still a future mitigation.

## 2026-06-13 — Release-only lockfile drift risk closed

- Risk observed on `v0.2.8`: manifest-only release version sync made Docker server release fail under `cargo fetch --locked`.
- Mitigation: `scripts/set-release-version.py --scope workspace` now updates `Cargo.lock` local workspace package versions and has a pytest contract plus isolated `cargo fetch --locked` simulation.
- Remaining risk: already-pushed `v0.2.8` remains a failed release attempt in Actions history; final release should be `v0.2.9` and memory should record both attempts.
