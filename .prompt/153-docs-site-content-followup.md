# 153 — Docs site P0 content follow-up (completed)

## Current context

`docs/` exists as a Docusaurus 3.10.1 TypeScript + Bun standalone docs site. Phase A scaffold is implemented. The 2026-06-08 follow-up completed the current P0 docs depth pass and zh-CN route mirror:

- English P0 pages now cover Overview, Installation, Quickstart, Seed demo data, Worker Tunnel, Workflows, Rust/Go/Java/Python/Node.js SDKs, Docker Compose, Kubernetes/Helm, Integrations, Configuration, and Troubleshooting.
- zh-CN counterparts exist for every current P0 route, fixing the previous Chinese 404 gap.
- The sidebar SDK section lists all current SDK families: Rust, Go, Java Spring Boot, Python, and Node.js.
- `.github/tests/docs_site_contract_test.py` now guards English evaluation depth, zh-CN file coverage, and zh-CN localized depth.

## Verification baseline

Local verification passed:

- `python3 .github/tests/docs_site_contract_test.py`
- `python3 scripts/check-source-size.py`
- `cd docs && bun install --frozen-lockfile`
- `cd docs && bun run docs:typecheck`
- `cd docs && bun run docs:build`
- `cd docs && bun run docs:serve -- --port 13031` plus curl smoke for `/zh-CN/docs/`, `/zh-CN/docs/getting-started/installation`, `/zh-CN/docs/sdks/rust`, `/zh-CN/docs/sdks/python`, `/zh-CN/docs/sdks/nodejs`, `/zh-CN/docs/deployment/kubernetes`, and `/zh-CN/docs/reference/troubleshooting`
- `python3 .github/tests/workflow_contract_test.py`
- workflow YAML parse
- `git diff --check`

Verification gap: `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24` timed out locally after 20s with no output during this docs-only slice; no workflow files were changed.

## Completed scope

1. Filled current P0 docs content from verified repository behavior.
2. Added Python and Node.js SDK docs instead of leaving the SDK list incomplete.
3. Expanded full current-route Chinese localization instead of starter summaries.
4. Updated `design/docs-site-build-plan.md`, `.memory/session-log.md`, `.memory/progress.md`, and `.memory/next.md`.

## Next prompt

Continue with `.prompt/154-docs-ci-and-reference-depth-followup.md`.
