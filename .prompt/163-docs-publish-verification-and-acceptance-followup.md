# 163 — Docs publish verification and acceptance follow-up

## Completed baseline

2026-06-10 completed the pre-website-migration docs follow-up after the `website/` -> `docs/` migration:

- Added English and zh-CN contributor runbooks for `scripts/management-trigger-e2e-smoke.sh`.
- Added English and zh-CN Kubernetes controller-specific runbooks for Nginx Ingress, Envoy Gateway, Traefik, and Gateway API.
- Added docs contract coverage for the runbooks so they remain source-backed by the real smoke script and Helm values/templates.
- Updated `docs/sidebars.ts`, `docs/static/search-index.json`, `docs/static/llms.txt`, and `docs/static/llms-full.txt` so the new pages are discoverable.
- Verified the actual Management trigger smoke: `TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh` passed with evidence under `.dev/reports/management-trigger-e2e-20260610T153458Z-230214`.

## Guardrails

- Do not reintroduce `website/` as a build module.
- Keep shared README/media assets under `assets/docs/`.
- Keep docs source-backed; do not invent controller behavior, SDK helper names, API endpoints, or Worker inbound services.
- Worker deployments remain outside the Helm chart and connect outbound to the Worker Tunnel.
- Use `bun` / `bunx` for docs and frontend commands.
- Functional/module acceptance phase: do not shrink scope; complete real missing behavior or record a concrete blocker.

## Suggested next slice

1. Trigger `Publish / Docker docs` through workflow_dispatch or a new release tag on a current ref, then record the digest for `yhyzgn/tikeo-docs`. Existing Docker Hub secrets are likely already configured because server/web Docker publish workflows have succeeded.
2. If no release tag should be created yet, use a manual image tag such as `main-<short-sha>` with `ref=main`; avoid reworking completed docs rename, runbook, search, SEO, or user-guide slices.

## Verification entrypoint

```bash
python3 .github/tests/workflow_contract_test.py
python3 .github/tests/docs_site_contract_test.py
python3 .github/tests/management_smoke_contract_test.py
python3 scripts/check-source-size.py
python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24
git diff --check
cd docs
bun install --frozen-lockfile
bun run docs:typecheck
bun run docs:build
cd ..
docker build -f docs/Dockerfile docs -t tikeo-docs:local
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```
