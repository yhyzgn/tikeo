# 162 — Docs module Docker and acceptance follow-up

## Completed baseline

2026-06-10 completed docs module migration and docs publishing/search/user-guide readiness:

- Docusaurus docs site module moved from `website/` to `docs/`.
- Old top-level `docs/assets/` media moved to `assets/docs/` and README links were updated.
- Added `docs/Dockerfile` plus nginx runtime config for a standalone static docs image.
- Added `.github/workflows/publish-docker-docs.yml` targeting Docker Hub repository `yhyzgn/tikeo-docs`.
- Main CI docs job now runs from `docs/`; split Docker validation includes server, web, and docs image builds with `push: false`.
- Docs publishing/search/SEO readiness now includes `TIKEO_DOCS_URL`, `TIKEO_DOCS_BASE_URL`, metadata, sitemap config, `robots.txt`, `search-index.json`, `static/img/tikeo-og.png`, `llms.txt`, and `llms-full.txt`.
- Source-backed user guides now exist in English and zh-CN for Dashboard, Jobs, Instances, Workers, Workflows, Scripts, Audit, and Settings.

## Guardrails

- Do not reintroduce `website/` as a build module.
- Keep shared README/media assets under `assets/docs/`, not under the Docusaurus module.
- Use `bun` / `bunx` for docs and frontend commands.
- Keep docs source-backed: UI docs must cite real `web/src/pages/*`, `web/src/routes.tsx`, API paths, or protocol/source artifacts.
- Functional/module acceptance phase: do not shrink scope; if missing/incomplete behavior is found, complete it production-grade or record a real blocker.

## Suggested next slice

1. Run or dry-run the docs image publish workflow when release tag/credentials are available, then record the Docker Hub digest for `yhyzgn/tikeo-docs`.
2. Add a contributor runbook for `scripts/management-trigger-e2e-smoke.sh`.
3. Add Kubernetes controller-specific docs for Nginx, Envoy, Traefik, and Gateway API with production values and smoke evidence.

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
```
