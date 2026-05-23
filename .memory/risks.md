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
- Web nginx 代理假设后端服务名为 `tikee`；Compose 与当前 K8s YAML 已保持该名称，若 Helm/生产命名变化，需要模板化 upstream。

- 009 dispatch loop 当前是单节点 first-available worker 策略，尚未实现 capability/tag 匹配、租约过期剔除、任务 ack 超时、重试、幂等锁和多 server 协调。
- TaskResult 当前只落实例最终状态，尚未持久化 worker_id、错误信息、执行耗时和日志。

- 010 tikee tick loop 使用内存 cursor，server 重启后可能重新计算到期触发；后续需要持久化 next_fire_at / last_fire_at 与分布式锁。
- CRON / Fixed Rate 当前只创建 pending instance，尚未实现 misfire 策略、时区配置、暂停/恢复、最大并发、任务堆积保护。

- 011 日志当前按实例分页骨架返回全部结果，尚未实现游标分页、日志压缩/归档、实时 SSE/WebSocket 流和敏感信息脱敏策略。
- TaskLog 持久化当前信任 worker 上报的 sequence/level/message，后续需要大小限制、速率限制和租户隔离。



## 2026-05-19 — 012 auth 风险

- 当前认证是开发期基础，不是生产安全方案；默认 `tikee_init/Tikee@2026!` 和静态 bearer token 仅用于本地与早期集成。
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

- 020 阶段已删除开发期 `tikee-init-token` 静态 bearer backdoor；后续风险转为需要完善正式 RBAC / OIDC / API Token 生命周期管理。
- 当前 session TTL 固定在代码中，后续应进入配置文件并支持 Redis 分布式实现。
- moka 本地缓存不是权威状态；多节点部署前必须实现 Redis 或事件驱动的跨节点撤销同步。

## 2026-05-20 — soft relation risk

- 全库禁止数据库外键后，关系完整性必须由 repository/service 和测试保障；后续删除父记录时要显式处理子记录清理。
- SQLite 兼容层会重建历史外键表；生产数据库迁移也必须遵守无外键策略并单独验证。

## 021 后续风险

- RBAC 当前是最小 `resource/action` 模型；API Token 已有 create/list/revoke、细粒度 `resource:action` scope、TTL 策略、rotate 与 namespace/app/worker_pool binding 基础；完整租户/app/worker-pool 管理 UI、租户隔离策略闭环和 OIDC 身份映射仍未实现。
- roles/permissions seed 当前主要覆盖内置角色；后续若开放角色管理 UI，需要补角色 CRUD、权限绑定审计和权限变更 session 失效。

## 2026-05-23 — Phase 3 closeout production gaps

- OIDC/SSO is fail-closed foundation only: authorize URL generation and callback shape exist, but no token exchange, JWKS verification, nonce/state persistence, user mapping, or session issuance from IdP identity is implemented.
- TLS/mTLS is config/status foundation only: TLS-enabled endpoints explicitly report `tls_pending_listener`; HTTP and Worker Tunnel still serve plaintext until real listener wiring lands.
- Script governance blocks unsafe releases and unverified approval/signature metadata, but full multi-level approval workflow, verified signatures/KMS, URL/File/Secret grants, and production release gates remain future work.
- Alerting has durable rules/events/recovery/summary, redacted channel readiness, production-guarded webhook/provider POST delivery, provider-specific Slack/DingTalk/Feishu/WeCom/PagerDuty JSON adapters, persisted delivery attempt history, local-loopback SMTP email delivery foundation, and bounded retry/DLQ processing plus ownership-gated background retry scheduling; production SMTP TLS/auth/secret handling and live external provider smoke remain future work.
- Observability has `/metrics`, metrics summary, Grafana template, dispatch queue pending-age and completed dispatch latency histograms, instance status/success-ratio, alert status, script-governance, and workflow/map-shard SLA Prometheus snapshots; live recording-rule validation and collector export smoke are still open.
