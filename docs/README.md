# Tikeo documentation site

This is the standalone Docusaurus 3 documentation site for Tikeo.

## Local development

The default build target is a standalone docs domain rooted at `/`, so Chinese routes such as `/zh-CN/docs/` work without extra hosting rewrites.

```bash
bun install --frozen-lockfile
bun start
# equivalent: bun run docs:dev
```

`bun start` / `bun run docs:dev` intentionally builds all locales first and then serves the generated site. Docusaurus `start` is single-locale by design; running it without `--locale zh-CN` does not load the Chinese route table, so switching to `/zh-CN/...` in that mode can render client-side “Page Not Found”. Use the hot-reload scripts only when you are editing one locale at a time:

```bash
bun run docs:dev:en
bun run docs:dev:zh
```

For GitHub Pages project hosting under `/tikeo/`, override the site URL and base URL:

```bash
TIKEO_DOCS_URL=https://yhyzgn.github.io TIKEO_DOCS_BASE_URL=/tikeo/ bun run docs:dev
```

## Verification

Default standalone-root build:

```bash
bun run docs:typecheck
bun run docs:build
bun run docs:serve -- --port 13030
```

Smoke URLs after default `docs:serve`:

```bash
curl -fsS http://127.0.0.1:13030/
curl -fsS http://127.0.0.1:13030/docs/
curl -fsS http://127.0.0.1:13030/zh-CN/docs/
curl -fsS http://127.0.0.1:13030/llms.txt
```

GitHub Pages project build:

```bash
TIKEO_DOCS_URL=https://yhyzgn.github.io TIKEO_DOCS_BASE_URL=/tikeo/ bun run docs:build
```

## Deployment configuration

| Environment variable | Default | Meaning |
| --- | --- | --- |
| `TIKEO_DOCS_URL` | `https://docs.tikeo.net` | Docusaurus `url`. Use the origin only, without a path. |
| `TIKEO_DOCS_BASE_URL` | `/` | Docusaurus `baseUrl`. Use `/tikeo/` only for GitHub Pages project hosting. |

If Chinese language switching returns “Page Not Found” on a static host, first verify whether the site is deployed at `/` or under a project subpath such as `/tikeo/`, then set `TIKEO_DOCS_BASE_URL` to the same path before building.

## Scope

The site currently covers the Phase A scaffold plus enriched P0 docs, complete current-route zh-CN localization, complete SDK list coverage, and full copy-paste deployment files/runbooks for single binary/systemd, Docker Compose, and Helm/Kubernetes. Final deployment provider configuration remains environment-specific.

## Container image

The docs site builds as a standalone nginx-served image, separate from the Web console image:

```bash
docker build -f docs/Dockerfile docs -t tikeo-docs:local
docker run --rm -p 8081:80 tikeo-docs:local
curl -fsS http://127.0.0.1:8081/healthz
curl -fsS http://127.0.0.1:8081/docs/
```

Release publishing uses `.github/workflows/publish-docker-docs.yml` and Docker Hub repository `yhyzgn/tikeo-docs`.
