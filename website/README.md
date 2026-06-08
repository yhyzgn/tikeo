# Tikeo documentation site

This is the standalone Docusaurus 3 documentation site for Tikeo.

## Local development

```bash
bun install --frozen-lockfile
bun run docs:dev
```

## Verification

```bash
bun run docs:typecheck
bun run docs:build
bun run docs:serve -- --port 13030
```

Smoke URLs after `docs:serve`:

```bash
curl -fsS http://127.0.0.1:13030/
curl -fsS http://127.0.0.1:13030/docs/
curl -fsS http://127.0.0.1:13030/zh-CN/docs/
curl -fsS http://127.0.0.1:13030/llms.txt
```

## Scope

The site currently covers the Phase A scaffold: Docusaurus config, bilingual routing, homepage, P0 docs pages, starter Chinese translations, and static `llms.txt` entrypoints. Deployment provider configuration is intentionally not hardwired yet.
