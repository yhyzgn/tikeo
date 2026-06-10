# 164 — Docs operator-grade manual follow-up

## Completed baseline

2026-06-10 upgraded the `docs/` Docusaurus site from README-adjacent content into an operator-grade manual.

Implemented and verified:

- English critical pages: `index`, `getting-started/installation`, `getting-started/quickstart`, `reference/configuration`, and Rust/Go/Java/Python/Node SDK pages now contain deep source-backed install/config/SDK/deploy/verification guidance.
- zh-CN critical pages mirror the required operational depth, including five SDK pages with dependency coordinates, WorkerConfig/Spring Boot defaults, minimal Worker patterns, management credentials, and现场验收 runbooks.
- `.github/tests/docs_site_contract_test.py` now rejects shallow critical docs and protects source-backed quickstart correctness.
- Quickstart defects found by verifier were fixed: bootstrap status uses `data.registrationOpen`, bootstrap registration exports `TOKEN`, and the temporary Node.js SDK trigger script is created/run from the repository root.
- Static discovery files `docs/static/search-index.json`, `docs/static/llms.txt`, and `docs/static/llms-full.txt` were refreshed for the deeper operator pages.
- `docs/nginx/default.conf` now disables absolute/port redirects so no-trailing-slash docs routes work behind local Docker port mapping and reverse proxies.

## Verification evidence

- `python3 .github/tests/docs_site_contract_test.py` passed (23 tests).
- `python3 .github/tests/workflow_contract_test.py` passed (15 tests).
- `python3 .github/tests/management_smoke_contract_test.py` passed.
- `cd docs && bun run docs:typecheck && bun run docs:build` passed.
- `python3 scripts/check-source-size.py` passed.
- `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24` passed.
- `git diff --check` passed.
- `docker build -f docs/Dockerfile docs -t tikeo-docs:local` passed.
- Docs container smoke passed on `/healthz`, `/docs/`, `/zh-CN/docs/`, no-trailing-slash `/docs/reference/configuration`, no-trailing-slash `/zh-CN/docs/sdks/rust`, and `/search-index.json`.

## Guardrails

- README remains a concise project introduction; the docs site owns operator depth. Do not copy README as docs content.
- Keep docs source-backed. Do not invent endpoints, config fields, SDK helper names, package coordinates, Worker inbound services, or controller behavior.
- Worker deployments remain outbound-only; Helm chart must not deploy business Workers or inbound Worker Services.
- Future docs/frontend commands use `bun`/`bunx`.
- Keep `docs/` as the docs-site module and `assets/docs/` for shared README/media assets.

## Suggested next slice

1. Trigger `Publish / Docker docs` on a current ref/tag and record the Docker Hub digest for `yhyzgn/tikeo-docs`.
2. If release tag policy is still in flight, use manual workflow dispatch with a non-release image tag such as `main-<short-sha>` and `ref=main`.
3. Do not reopen completed docs depth/rename/runbook work unless new source-backed gaps are found.
