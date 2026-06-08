# 153 — Docs site content follow-up

## Current context

`website/` now exists as a Docusaurus 3.10.1 TypeScript + Bun standalone docs site. Phase A scaffold is implemented: homepage, navbar/footer, sidebar IA, P0 English starter pages, starter `zh-CN` translations, release-note blog entry, static `llms.txt` / `llms-full.txt`, and build/typecheck scripts.

## Verification baseline

Local verification passed:

- `python3 .github/tests/docs_site_contract_test.py`
- `python3 scripts/check-source-size.py`
- `cd website && bun install --frozen-lockfile`
- `cd website && bun run docs:typecheck`
- `cd website && bun run docs:build`
- `cd website && bun run docs:serve -- --port 13030` plus curl smoke for `/`, `/docs/`, `/zh-CN/docs/`, `/docs/getting-started/quickstart`, and `/llms.txt`

## Next recommended slice

1. Fill Phase B English docs content for the P0 pages from verified repository behavior: Overview, Installation, Quickstart, Worker Tunnel, Workflows, Rust/Go/Java SDK pages, Docker Compose, Kubernetes/Helm, Configuration, Troubleshooting.
2. Add docs-site CI only after deciding whether docs build should run in main CI or a docs-specific workflow.
3. Expand Chinese localization after English P0 content is stable; avoid partial machine-summary translations.
4. Keep deployment provider configuration separate until the final docs hosting target is chosen.

## Guardrails

- Do not advertise unverified behavior or fake SDK quickstarts.
- Keep Python and Node.js content tied to actual implemented SDK/demo evidence.
- Do not manually copy the entire architecture design document into one large page.
- Keep `website/build`, `.docusaurus`, and `node_modules` ignored.
