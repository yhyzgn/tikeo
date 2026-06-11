# Latest completed slice

- 2026-06-12: Notification Center workflow notification acceptance hardening is complete locally. Workflow notification nodes now use registered Notification Center channel/template refs, reject legacy raw target-only shapes, validate channel/template existence and enabled state in dry-run/create/update/validate, and fail closed at runtime if refs go stale. Alert `alert.firing` / `alert.recovered` docs now match implemented Notification Center materialization.

# Next Work

## Current priority direction

当前仍是功能/模块测试验收与发布阶段，不收缩、不臆造。Notification Center 的渠道、策略、模板、消息、投递、provider schema、告警事件物化、普通 Job 生命周期物化、Workflow notification inline channel/template refs 已落地并通过本地验收。Alert rule 自动迁移/dual-write、真实 channel test-send endpoint、外部 SaaS live smoke、delivery idempotency lease hardening 仍不能说成完成。

## Immediate next slice after this release

1. Commit and push the current Notification Center/workflow notification hardening.
2. Create/push the next `v0.2.x` tag and monitor GitHub Actions until main CI/Coverage and tag-triggered Release/Docker/SDK workflows are green, including docs image publication to `yhyzgn/tikeo-docs` when the docs workflow runs.
3. If docs human-operator manual work is not already in the pushed branch/tag, continue/re-run the docs-site humanization task immediately after this notification phase: human-readable deployment/configuration/SDK/integration runbooks, not README rehashes and not AI-facing notes.
4. Implement alert-rule dual-write/backfill to Notification Center policies while preserving legacy alert APIs.
5. Add delivery lease/idempotency hardening so crash recovery avoids both lost notifications and duplicate provider calls.
6. Add real channel `:test` endpoint only when it persists attempts and redacts results; until then `supportsTestSend=false` is correct.

## Current verified baseline

- Notification Center baseline: explicit schema/repositories, routes/OpenAPI, config defaults, background worker, Web page, docs, RBAC/menu seed, job lifecycle materialization, alert event materialization, workflow notification node materialization, redaction, generic retry/DLQ, and provider delivery for webhook-style/Slack/DingTalk/Feishu/WeCom/PagerDuty/Email/plugin webhook-compatible providers.
- Template baseline: `notification_templates` has an explicit SeaORM migration/entity/repository, CRUD/list/get/delete API, `/api/v1/notification-templates/{id}/render` preview endpoint, safe token replacement, provider/message-type validation, Web template drawer/preview, and policy template AutoComplete options restricted to enabled stored templates matching selected channel providers.
- Workflow notification baseline: Web uses Notification Center channel/template selectors; backend validates refs in dry-run/create/update/validate; runtime inline policy materialization fails closed if refs go stale.
- Alerting baseline remains compatible: alert rules/events/delivery attempts/retry UI still exist; Alerting owns incident semantics while Notification Center owns reusable outbound delivery.
- Docs site module baseline remains `docs/`, with Docker publish workflow targeting `yhyzgn/tikeo-docs`.

## Standing constraints

- Functional/module testing acceptance phase: do not shrink scope; if anything missing/incomplete/untested/hallucinated is found, fill it production-grade or record a real blocker. Keep durable context fresh and source-backed.
- Alerts = rules/events/incidents; Notifications = channels/templates/policies/messages/delivery. Inbound webhook event sources are job triggers, not outbound notification channels.
- Never leak webhook tokens, SMTP passwords, PagerDuty keys, header credentials, or secret ref values in API responses/UI/docs/logs.
- Worker 重要可见性状态不得只在内存。
- 禁止约定命名匹配；必须使用结构化字段、labels 或 structuredCapabilities。
- 中文 i18n 必须完整中文，英文 i18n 必须英文，不要中英混合 label。
- 新 schema 变更必须进入显式 SeaORM migration；不得在 `connect_and_migrate` 后挂未记录的兼容补丁。
- Helm chart 不能部署业务 Worker 或创建业务 Worker 入站 Service；Worker 只能主动出站连接 Tikeo Worker Tunnel。
- 源文件 <=1500 行；`mod.rs` / `lib.rs` 等入口文件只做声明和 re-export。
- Web/frontend/docs package management and command execution must use `bun` / `bunx` unless explicitly overridden.
