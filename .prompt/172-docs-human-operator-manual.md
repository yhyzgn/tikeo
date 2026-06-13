# Docs human operator manual continuation

## Goal
Docs must remain a human-readable operating manual, not an AI handoff or README rehash. Docusaurus is still sufficient; the main work is information architecture, depth, examples, verification, and zh-CN parity.

## Completed in this slice
- Reframed docs navigation into task paths: Getting Started, Core Concepts, User Guide, Deployment & Operations, SDKs & API Integrations, Develop and extend, Reference.
- Added production-focused docs:
  - `docs/docs/deployment/production.md`
  - `docs/docs/integrations/sdk-and-api.md`
  - `docs/docs/reference/configuration-cookbook.md`
  - `docs/docs/development/overview.md`
- Added zh-CN mirrors for those pages under `docs/i18n/zh-CN/docusaurus-plugin-content-docs/current/`.
- Rebuilt homepage as a role/task portal with local quickstart, production deployment, app integration, configuration cookbook, Notification Center, SDK manuals, development guide, and troubleshooting paths.
- Updated docs search and LLM entrypoint surfaces for the new pages.
- Preserved docs Docker image contract for `yhyzgn/tikeo-docs` and documented the docs image in the production deployment guide.

## Verification evidence
- `python3 .github/tests/docs_site_contract_test.py` passed.
- `bun run --cwd docs docs:typecheck` passed.
- `bun run --cwd docs docs:build` passed for en and zh-CN.
- `python3 scripts/check-source-size.py` passed.
- `git diff --check` passed.
- `docker build -f docs/Dockerfile docs -t tikeo-docs:local-human-manual` passed.
- Container smoke passed: `/healthz`, `/docs/`, and `/zh-CN/docs/`.

## Next quality bar
Future docs work should keep adding human paths with prerequisites, exact commands, expected observations, troubleshooting, and production checklist. Do not add public docs language like internal handoff notes. If new code changes config/API/SDK/deploy behavior, update both user path and reference page plus zh-CN priority mirrors.
