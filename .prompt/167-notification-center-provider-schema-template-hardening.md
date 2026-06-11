# 167 — Notification Center provider schema/template hardening follow-up

## Context

The 2026-06-11 follow-up slice strengthened the Notification Center channel drawer, built-in provider schemas, and first-class reusable notification templates after review. It continues `.prompt/166-notification-center-templates-alert-workflow-followup.md` and `design/notification-center-alerting-plan.md`.

## Completed in this slice

- Channel drawer now uses provider metadata for linked config, secret refs, message type, and template fields.
- Edit mode no longer blindly resubmits redacted config or empty secret refs. Existing config/secret refs are preserved unless the operator explicitly enables replacement toggles.
- Scope controls are cascading and clear dependent app/worker-pool/secret selections when scope context changes.
- Backend channel validation now checks scope field requirements and namespace/app/worker-pool consistency.
- Backend built-in metadata now separates `requiredConfigKeys` from `requiredTargetKeys` so secret-ref-first target configuration is not contradicted by metadata.
- Built-in provider template metadata and runtime rendering now cover:
  - Slack `text`, `blockKit`, `attachments`, optional `threadTs` -> `thread_ts`.
  - DingTalk `text`, `markdown`, `link`, `actionCard` single button or `btns`, and `feedCard`, plus signing.
  - Feishu `text`, `post`, `image`, `share_chat`, `interactive`, plus signing.
  - WeCom `text`, `markdown`, `markdown_v2`, `image`, `news`, `file`, `voice`, `template_card`.
  - PagerDuty `trigger`, `acknowledge`, `resolve`, plus `client`, `client_url`, `links`, `images`, `custom_details`.
  - Generic webhook JSON and email subject/body template override.
- Built-in provider create/update validation now rejects unsupported `messageType`, missing required template fields, and raw secret config keys for built-in secret material such as PagerDuty routing keys and bot signing secrets.
- First-class `notification_templates` are implemented through explicit SeaORM migration/entity/repository, CRUD/list/get/delete HTTP routes, OpenAPI registration, safe token validation, render-preview endpoint, and Web template drawer/preview.
- Policy `templateRef` options are generated from enabled stored templates and filtered by selected channel providers. Runtime materialization loads enabled stored templates by id or `templateKey`, renders safe tokens, stores rendered output under `payload.template`, and provider delivery prefers that stored template over channel inline defaults.
- English and zh-CN Notification Center docs now describe provider schemas, target refs, message type coverage, stored template APIs, render preview, and official/standard reference sources.

## Fresh verification from the current session

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features -- --test-threads=1
cargo build --workspace --all-features
cd web && bun run lint && bun run typecheck && bun test src && bun run build
cd docs && bun run docs:typecheck && bun run docs:build
python3 .github/tests/docs_site_contract_test.py
python3 .github/tests/workflow_contract_test.py
python3 .github/tests/management_smoke_contract_test.py
python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24
python3 scripts/check-source-size.py
git diff --check
```

All commands above passed in the current session before commit. Web build still emits the known large Ant Design chunk warning only.

## Do not overclaim

Still not implemented unless a later slice adds it:

1. `POST /api/v1/notification-channels/{id}:test`; metadata must remain `supportsTestSend=false` until implemented with persisted attempts and redacted results.
2. Alert rule automatic migration/dual-write to Notification Center policies.
3. Workflow `notification` node migration is now implemented after this handoff: use registered Notification Center channel/template refs with fail-closed validation. Future work should focus on richer UI interaction tests and policy-mode observability, not raw target migration.
4. Live SaaS smoke against Slack/DingTalk/Feishu/WeCom/PagerDuty; credentials are not present.
5. Email HTML/MIME delivery runtime. The drawer stores an HTML template shape, but runtime still uses the existing text/plain SMTP adapter.
6. Template policy impact preview/cascade guard on delete. `templateRef` is a soft link; deleting templates should be operationally guarded in a future slice.

## Recommended next slice

Migrate alert rules and workflow notification nodes onto the now-real Notification Center template/channel primitives. Keep files under 1500 lines and split route/service modules before adding more notification API surface.
