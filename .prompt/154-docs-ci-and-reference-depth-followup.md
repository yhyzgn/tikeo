# 154 — Docs CI, publish target, and reference-depth follow-up

## Current baseline

The standalone docs site has a verified P0 content/localization/deployment baseline:

- Docusaurus 3.10.1 TypeScript + Bun app in `website/`.
- Default docs deployment target is a standalone-root site (`baseUrl=/`) so `/zh-CN/` works without subpath hosting rewrites.
- GitHub Pages project hosting is supported with `TIKEO_DOCS_URL=https://yhyzgn.github.io` and `TIKEO_DOCS_BASE_URL=/tikeo/`; default builds stay root-based for standalone docs hosting.
- English P0 docs have contract-enforced minimum evaluation depth.
- zh-CN P0 docs exist for every current P0 route and have contract-enforced localized depth.
- SDK docs cover Rust, Go, Java Spring Boot, Python, and Node.js.
- Deployment docs include copy-paste runbooks for single binary/systemd, full Docker Compose SQLite/PostgreSQL/MySQL YAML files, Helm dev/prod/TLS/ops overlays, and configuration parameters.
- Local default `/tikeo/` and custom root `/` builds/serve smokes are green for zh-CN language-switch routes.

## Recommended next slice

1. Add docs verification to CI.
   - Decide whether to extend main CI or create a docs-specific workflow.
   - Minimum commands: `python3 .github/tests/docs_site_contract_test.py`, `cd website && bun install --frozen-lockfile`, `bun run docs:typecheck`, and `bun run docs:build`.
   - For standalone docs deployment, keep the default `TIKEO_DOCS_BASE_URL=/`; for GitHub Pages project hosting, set `TIKEO_DOCS_BASE_URL=/tikeo/`.
2. Select and document final docs hosting.
   - If using a standalone domain: verify `/zh-CN/...` after deployment.
   - If using GitHub Pages project hosting: set `TIKEO_DOCS_URL=https://yhyzgn.github.io`, `TIKEO_DOCS_BASE_URL=/tikeo/`, and verify `/tikeo/zh-CN/...`.
3. Expand source-backed reference depth.
   - SDK overview and cross-language parity guide.
   - User guide pages for Dashboard, Jobs, Instances, Workers, Workflows, Scripts, Audit, and Settings.
   - Generated or source-derived OpenAPI/protobuf references.
   - Configuration/environment variable matrix generated from committed config structures or examples.

## Guardrails

- Do not advertise unverified runtime behavior.
- Keep Python/Node docs tied to actual `sdks/*` and `examples/*` commands.
- Do not manually invent API schemas if OpenAPI/protobuf generation can own the reference.
- Keep zh-CN pages complete for any new P0 sidebar route added.
- Keep deployment commands copy-pasteable and state exactly which values must be replaced for production.

## Verification entrypoint

Before committing any next docs slice, run:

```bash
python3 .github/tests/docs_site_contract_test.py
python3 scripts/check-source-size.py
cd website
bun install --frozen-lockfile
bun run docs:typecheck
bun run docs:build
```

For route/baseUrl changes, also run `bun run docs:serve` and curl affected English and zh-CN routes under both `/tikeo/` and `/` when relevant.
