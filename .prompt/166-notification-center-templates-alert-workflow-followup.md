# 166 — Notification Center templates, alert migration, and workflow notification follow-up

## Context

The 2026-06-11 Notification Center implementation slice is complete and locally verified. It added reusable notification channels/policies/messages/delivery attempts, job lifecycle materialization, generic provider delivery/retry/DLQ, Web `/notifications`, RBAC/menu seed, and English/zh-CN docs.

Canonical context:

- `design/notification-center-alerting-plan.md`
- `design/tikeo-architecture-design.md` notification/alerting roadmap
- `.memory/session-log.md` entry `2026-06-11 — Notification Center implementation and acceptance hardening`
- `.memory/next.md`
- `.memory/risks.md`

## Current implemented baseline

Implemented:

- `notification_channels`, `notification_policies`, `notification_messages`, `notification_delivery_attempts` schema/entities/repositories.
- `/api/v1/notification-channel-types`, channel CRUD, policy CRUD/validation, message list, delivery attempt list, queue status, retry-due.
- Background generic notification delivery worker configured by `[notification_delivery]`.
- Web `/notifications` page with provider metadata, channel/policy CRUD, validation, queue/DLQ, recent messages.
- Job lifecycle events: `job_instance.succeeded`, `failed`, `partial_failed`, `cancelled`, `retry_scheduled`, `retry_exhausted`, `no_eligible_worker`, `script_governance_failure`.
- Safety fixes: no `secretRefsJson` serialization, URL/key/header redaction, `secretRefs.authorization` and `secretRefs.headers.*` runtime injection, email `secretRefs.password` and SMTP alias handling, env-only secretRef docs/UI, viewer menu seed.
- Retry semantics: scheduled retries emit `retry_scheduled`; non-retrying failures emit `failed`; actual exhausted retries emit `retry_exhausted`.

Verified locally before handoff:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features -- --test-threads=1`
- `cargo build --workspace --all-features`
- `cargo run -- --help`
- `cd web && bun run lint && bun run typecheck && bun test src && bun run build`
- `cd docs && bun run docs:typecheck && bun run docs:build`
- `python3 .github/tests/workflow_contract_test.py`
- `python3 .github/tests/docs_site_contract_test.py`
- `python3 .github/tests/management_smoke_contract_test.py`
- `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24`
- `python3 scripts/check-source-size.py`
- `git diff --check`

## Do not overclaim

Not yet implemented:

1. `notification_templates` table/API/render dry-run.
2. Real channel test-send endpoint. Current metadata must remain `supportsTestSend=false` until implemented with persisted attempts and redacted results.
3. Alert rule backfill/dual-write from `alert_rules.channels_json` into `notification_policies(owner_type='alert_rule')`.
4. Workflow `notification` node runtime/UI migration from raw `channel/target/template` to registered channel/template refs.
5. Delivery lease/idempotency hardening. Current ordering is at-least-once and may duplicate if a process crashes after result row insertion but before previous attempt consumption.

## Recommended next slice

1. Implement `notification_templates` first:
   - Add explicit SeaORM migration/entity/repository.
   - Routes: `GET/POST /api/v1/notification-templates`, `GET/PATCH/DELETE /api/v1/notification-templates/{id}`, and `POST /api/v1/notification-templates/{id}:render`.
   - Use a safe token replacer only; no arbitrary template engine without security review.
   - Web: template list/editor/preview and policy template selector.
   - Tests: storage CRUD, HTTP envelope/RBAC/redaction, render dry-run with safe sample context, source-size check.

2. Then migrate Alerting:
   - Preserve existing alert routes and `channels_json` compatibility.
   - Create/backfill `notification_policies(owner_type='alert_rule')` from existing alert rule channels, or bridge alert events into Notification Center while keeping legacy attempts until migration is complete.
   - Tests must prove existing alert delivery/status tests stay green and new notification policy path works.

3. Then migrate Workflow notification nodes:
   - Replace raw `channel/target/template` with channel/template selectors.
   - Default behavior remains non-blocking; blocking delivery must be explicit.
   - Tests must show workflow notification node creates `notification_messages` and attempts.

## Constraints

- No hidden startup schema patches; all schema changes go through explicit migrations.
- Source files must remain <=1500 lines; split by responsibility.
- All HTTP business APIs return `{code,message,data}`.
- Web/docs commands use `bun`/`bunx`.
- Never leak webhook tokens, SMTP passwords, PagerDuty keys, header credentials, or secret ref values in API/UI/docs/logs.
- Keep Alerts and Notifications vocabulary strict: Alerts = incidents/rules/events; Notifications = channels/templates/policies/messages/delivery.
