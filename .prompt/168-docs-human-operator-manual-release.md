# 168 — Docs human operator manual release follow-up

## Context

The 2026-06-12 docs slice rewrote the Docusaurus `docs/` module into a human-readable operator manual. This follows the user's acceptance-stage directive: do not shrink scope; if docs are incomplete, rough, AI-facing, or not enough to deploy/configure/integrate from, make them complete and verify with real evidence.

## Completed in this slice

- Reworked priority English and zh-CN docs into operator manuals with prerequisites, verification, troubleshooting, and production checklists.
- Expanded getting-started, seed demo data, deployment, SSE realtime, integrations, configuration, troubleshooting, SDK, user-guide, alerts, notifications, and Notification Center content.
- Notification docs now give a chainable `channel → template → policy → event → delivery` runbook with `CHANNEL_ID`, `TEMPLATE_ID`, `POLICY_ID`, `jq -r '.data.id'`, `secretRefs`, retry/DLQ, and `supportsTestSend=false` until a real persisted redacted test endpoint exists.
- Notification Center reference provider table rendering was fixed after verifier review, and contract tests now catch table interruption before `pagerduty`/`email`/plugin rows.
- Public docs now reject internal AI handoff terms, README-rehash shallowness, `http://0.0.0.0` client URLs, and placeholder notification IDs.

## Fresh local verification

```bash
python3 .github/tests/docs_site_contract_test.py
python3 .github/tests/workflow_contract_test.py
python3 .github/tests/management_smoke_contract_test.py
python3 scripts/check-source-size.py
git diff --check
grep -R "source-backed\|source-derived\|docs slice\|hallucinated\|memory/prompt\|prompt handoff\|Contributor\|源码事实\|http://0.0.0.0\|notification-channel-example\|notification-policy-example" -n docs/docs docs/i18n/zh-CN/docusaurus-plugin-content-docs/current docs/src || true
cd docs && bun run docs:typecheck && bun run docs:build
docker build -f docs/Dockerfile docs -t tikeo-docs:local
# container smoke on 127.0.0.1:13036: /healthz, /docs/, /zh-CN/docs/, /docs/reference/notification-center, /search-index.json
```

All commands above passed locally before commit in the 2026-06-12 session.

## Do not overclaim

- Live Docker Hub publication is only complete after the tag-triggered `Publish / Docker docs` workflow succeeds and `yhyzgn/tikeo-docs` digest is visible.
- Live external SaaS notification delivery remains credential-gated; docs describe configuration and local/provider contracts but do not claim live Slack/DingTalk/Feishu/WeCom/PagerDuty smoke.
- `POST /api/v1/notification-channels/{id}:test` is still not implemented; `supportsTestSend=false` is correct.
- Alert-rule dual-write/backfill remains future work. Workflow notification-node raw-target migration has since been implemented with registered channel/template refs and fail-closed validation.

## Next action

Commit the current Notification Center/workflow notification hardening plus docs/manual updates, push `main`, create the next `v0.2.x` tag, then monitor GitHub Actions until main CI/Coverage and tag-triggered release/Docker/SDK workflows are green.
