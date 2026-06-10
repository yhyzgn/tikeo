# Tikeo Docs Site Build Plan

Status: Phase B/C P0 docs, zh-CN route mirror, and copy-paste deployment docs implemented
Last refreshed: 2026-06-08
Owner: Tikeo maintainers
Scope: Standalone documentation site design plus current `docs/` scaffold, enriched P0 docs, full P0 zh-CN route mirror, SDK coverage for Rust/Go/Java/Python/Node.js, subpath-safe language switching, and copy-paste deployment docs for binary/systemd, Compose, and Helm. Deployment target selection remains separate.

## 1. Goal

Build a standalone documentation site that turns GitHub interest into concrete evaluation:

- a first-time visitor understands Tikeo in under 30 seconds;
- an engineer can run Server + Web + one Worker demo in under 10 minutes;
- an operator can compare deployment paths and production risks without reading source code;
- SDK users can implement workers in Rust, Go, Java, Python, or Node.js from one consistent mental model;
- maintainers can keep docs versioned, searchable, bilingual, and suitable for LLM ingestion.

The site should complement, not replace, `README.md`. The README remains the GitHub first fold; the docs site owns depth, workflows, API/reference detail, and deploy/operator guidance.

## 2. Reference model: Hermes-style documentation IA

Reference observed: `https://hermes-agent.nousresearch.com/docs`.

Usable patterns to borrow:

- Docusaurus-style docs landing page with a concise product pitch, quick start CTA, and sidebar-driven information architecture.
- Top-level navigation separating Docs, feature catalogs, and download/install entry points.
- Bilingual routing (`en` and Simplified Chinese) with language switcher.
- A progressive navigation hierarchy: Getting Started, User Guide, Features, Integrations, Guides, Developer Guide, Reference.
- LLM-readable docs exports such as `llms.txt` / `llms-full.txt` for agent and search ingestion.
- Dedicated reference pages for commands, configuration, tools/features, environment variables, and FAQ/troubleshooting.

Tikeo should not copy Hermes content. It should copy the structure discipline: fast onboarding first, conceptual clarity second, then deep reference material.

## 3. Recommended site stack

Use **Docusaurus 3** for the first docs site.

Rationale:

- The reference site is Docusaurus-based, so the desired navigation, i18n, sidebar, docs versioning, and generated metadata patterns are proven in the target style.
- Tikeo already has a React/TypeScript web app, so a React-based docs stack stays familiar.
- Docusaurus has mature docs versioning, MDX, admonitions, code tabs, generated sidebars, edit links, Algolia DocSearch compatibility, and deployment flexibility.
- It can be deployed later to GitHub Pages, Cloudflare Pages, Vercel, Netlify, or any static hosting target.

Alternative candidates:

- **VitePress**: simpler and fast, but less aligned with the Hermes reference and less feature-rich for versioned docs/i18n/reference depth.
- **Fumadocs**: modern and attractive, but introduces Next.js hosting assumptions and more app-level decisions than needed for a docs-only first release.

Decision for first release: Docusaurus 3, TypeScript config, MDX docs, bilingual content, static build output.

2026-06-08 update: Phase B now guards P0 documentation quality through `.github/tests/docs_site_contract_test.py`. The contract requires every P0 English page to have evaluation depth, every P0 route to have a zh-CN counterpart, zh-CN pages to contain real localized depth, and SDK docs to cover Rust, Go, Java Spring Boot, Python, and Node.js.

2026-06-08 deployment update: docs builds are now safe for GitHub Pages project hosting by default (`/tikeo/`) while allowing custom-root deployment through `TIKEO_DOCS_URL` and `TIKEO_DOCS_BASE_URL`. Deployment P0 docs include copy-paste runbooks and parameter references for single binary/systemd, Docker Compose SQLite/PostgreSQL/MySQL, Helm dev/prod/TLS/ops overlays, and runtime configuration.

## 4. Information architecture

### 4.1 Top navigation

```text
Home
Docs
SDKs
Integrations
Blog / Releases
GitHub
Language: English / 简体中文
```

Notes:

- `Home` is the marketing landing page for the docs domain.
- `Docs` opens the docs landing page.
- `SDKs` jumps to multi-language SDK entry pages.
- `Integrations` highlights platform ecosystem material: OpenAPI, gRPC, Kubernetes, Terraform, Prometheus, OIDC, alert channels.
- `Blog / Releases` can start as a changelog-style release notes section.
- `GitHub` links to the repository.

### 4.2 Docs sidebar structure

```text
Docs
├─ Overview
│  ├─ What is Tikeo?
│  ├─ Why not XXL-Job / PowerJob?
│  ├─ Architecture at a glance
│  └─ Evaluation checklist
├─ Getting Started
│  ├─ Installation
│  ├─ Quickstart: Server + Web + Worker
│  ├─ Seed demo data
│  ├─ First scheduled job
│  ├─ First workflow
│  └─ Learning path
├─ Core Concepts
│  ├─ Server, Worker, and Worker Tunnel
│  ├─ Jobs, instances, attempts, and logs
│  ├─ Schedules and trigger events
│  ├─ Execution modes
│  ├─ Workflow DAG model
│  ├─ Dynamic scripts and sandboxing
│  ├─ Tenants, namespaces, apps, and worker pools
│  ├─ Security model
│  └─ Observability model
├─ User Guide
│  ├─ Dashboard
│  ├─ Jobs
│  ├─ Instances and logs
│  ├─ Workflows canvas
│  ├─ Workers and dispatch queue
│  ├─ Scripts and versions
│  ├─ Users, roles, and RBAC
│  ├─ Audit log
│  ├─ Settings
│  └─ CLI
├─ SDKs
│  ├─ SDK overview
│  ├─ Rust Worker SDK
│  ├─ Go Worker SDK
│  ├─ Java Spring Boot Starter
│  ├─ Python Worker SDK
│  ├─ Node.js / TypeScript Worker SDK
│  └─ Cross-language parity rules
├─ Deployment
│  ├─ Deployment overview
│  ├─ Single binary
│  ├─ Docker Compose with SQLite
│  ├─ Docker Compose with PostgreSQL
│  ├─ Docker Compose with MySQL
│  ├─ Kubernetes manifests
│  ├─ Helm chart plan
│  ├─ External database configuration
│  ├─ TLS / mTLS / secrets
│  ├─ Production hardening checklist
│  └─ Upgrade and rollback
├─ Integrations
│  ├─ HTTP API and OpenAPI
│  ├─ gRPC and protobuf
│  ├─ Prometheus and Grafana
│  ├─ OpenTelemetry
│  ├─ OIDC providers
│  ├─ Kubernetes Operator plan
│  ├─ Terraform Provider plan
│  └─ Alert channels
├─ Guides and Tutorials
│  ├─ Migrate from XXL-Job
│  ├─ Migrate from PowerJob
│  ├─ Build a multi-language worker pool
│  ├─ Create a workflow with approval and retry
│  ├─ Govern dynamic scripts safely
│  ├─ Run Tikeo across VPCs or clusters
│  ├─ Debug a stuck instance
│  └─ Prepare a production rollout
├─ Developer Guide
│  ├─ Contributing
│  ├─ Repository architecture
│  ├─ Protocol design
│  ├─ Storage and migrations
│  ├─ Scheduler internals
│  ├─ Worker tunnel internals
│  ├─ Workflow engine internals
│  ├─ Web UI architecture
│  ├─ Testing strategy
│  └─ Release process
└─ Reference
   ├─ CLI commands
   ├─ Configuration file
   ├─ Environment variables
   ├─ HTTP API reference
   ├─ gRPC reference
   ├─ Permission matrix
   ├─ Metrics reference
   ├─ Audit event reference
   ├─ Database compatibility matrix
   ├─ Troubleshooting
   └─ FAQ
```

### 4.3 Chinese mirror

Use Docusaurus i18n instead of maintaining an unrelated Chinese site.

Recommended locale paths:

```text
/docs/                 # English default
/docs/zh-CN/ or /zh-Hans/  # Simplified Chinese
```

Prefer `zh-CN` if Tikeo already uses that locale in Web UI and README naming; prefer `zh-Hans` only if the docs stack standardizes on Docusaurus defaults. Pick one and keep it consistent across URLs, metadata, and language switcher labels.

## 5. Proposed repository layout

When implementation begins, add a standalone docs app. Recommended location:

```text
docs/
├─ package.json
├─ bun.lock
├─ Dockerfile
├─ nginx/
├─ docusaurus.config.ts
├─ sidebars.ts
├─ docs/
│  ├─ index.md
│  ├─ getting-started/
│  ├─ concepts/
│  ├─ user-guide/
│  ├─ sdks/
│  ├─ deployment/
│  ├─ integrations/
│  └─ reference/
├─ blog/
├─ src/
│  ├─ css/custom.css
│  └─ pages/index.tsx
├─ static/
│  ├─ img/
│  ├─ llms.txt
│  ├─ llms-full.txt
│  ├─ robots.txt
│  └─ search-index.json
└─ i18n/
   └─ zh-CN/
      ├─ docusaurus-plugin-content-docs/current/
      ├─ docusaurus-plugin-content-blog/
      └─ code.json

assets/docs/
└─ shared README and docs media assets
```

Why `docs/` is now the docs-site module:

- The documentation site is a first-class buildable module, so the repository root `docs/` now contains the Docusaurus app.
- Shared README/media assets moved to `assets/docs/` to avoid mixing static-site source with generic project assets.
- The old `website/` directory name is retired so docs CI, Docker publishing, and local commands all use the same `docs/` module boundary.

## 6. Landing page content model

The docs home page should be a product explanation plus conversion path.

Recommended first fold:

```text
Tikeo
Rust-native orchestration for jobs, workflows, workers, and governed scripts.

No exposed worker ports. Multi-language workers. Workflow canvas.
Governed scripts. Audit-ready execution evidence.

[Get started] [View architecture] [Run a worker] [GitHub]
```

Below the fold:

1. Demo GIF or short MP4 loop from `assets/docs/tikeo-console-tour.gif` or the promo video artifact.
2. Four capability cards:
   - Worker Tunnel
   - Workflow DAG Canvas
   - Multi-language SDKs
   - Security and Audit
3. Quickstart command block.
4. Architecture diagram from `assets/docs/tikeo-architecture.en.svg`.
5. Comparison strip: Tikeo vs XXL-Job vs PowerJob.
6. Deployment choices: binary, Docker, Kubernetes, Helm roadmap.
7. Footer links: GitHub, releases, security, roadmap, license.

## 7. Content ownership map

| Source | Use in docs site | Notes |
|---|---|---|
| `README.md` | Overview, product pitch, quickstart | Do not duplicate every matrix; link to deeper pages. |
| `README.zh-CN.md` | Chinese overview | Keep terminology aligned with Web UI i18n. |
| `design/tikeo-architecture-design.md` | Architecture, concepts, internals, migration guides | Split into smaller pages; keep original design doc as source-of-truth. |
| `deploy/README.md` | Deployment section | Convert into Docker/Kubernetes/Helm pages. |
| `sdks/README.md` | SDK overview | Expand into language-specific pages. |
| `examples/README.md` | Tutorials and runnable demos | Only document runnable demos as runnable. |
| `.memory/commands.md` | Verification and local run commands | Use only stable, current commands. |
| `assets/docs/*` | Architecture diagrams and demo media | Reuse current assets; avoid bloating repository. |

## 8. Documentation quality rules

- Every getting-started page must include exact commands, expected output, and next link.
- Every SDK page must show a minimal worker, registration behavior, execution handler, and verification command.
- Every deployment page must state supported database, required ports, environment variables, health checks, copy-paste commands, cleanup/rollback notes, and production replacement points.
- Every security page must distinguish current implemented behavior from roadmap items.
- Document Python/Node runnable demos only from verified SDK/demo commands and keep capability claims tied to CI or local evidence.
- Keep English as the default site language for international promotion; provide Chinese pages as complete translations, not partial summaries.
- Prefer diagrams, tables, and short command blocks over long prose.
- Add “last verified against commit” frontmatter or footer for operational pages once the docs build pipeline exists.

## 9. Search, SEO, and LLM-readability

### Search

Initial release:

- local search plugin or generated search index if Algolia DocSearch is not available yet;
- clear page titles and descriptions;
- stable slugs.

Later release:

- Algolia DocSearch after the public docs domain is live and crawlable.

### SEO metadata

Each major page should define:

```yaml
title: ...
description: ...
slug: ...
keywords:
  - rust scheduler
  - workflow orchestration
  - worker tunnel
  - distributed job scheduler
```

### LLM-readable exports

Add generated endpoints or static files:

```text
/llms.txt       # short index and high-value page list
/llms-full.txt  # concatenated docs snapshot for agents/search ingestion
```

These should be generated during build rather than manually maintained after the first release.

## 10. Visual direction

Use a technical, trustworthy, operator-friendly style:

- dark/light mode support;
- restrained Rust/orchestration visual identity;
- strong architecture diagrams and terminal examples;
- status badges only for verified artifacts;
- consistent feature-card iconography;
- high contrast and readable code blocks;
- no heavy animation that distracts from documentation.

Recommended homepage visual assets:

- existing console tour GIF;
- architecture SVG;
- short workflow canvas screenshot;
- worker tunnel sequence diagram;
- deployment topology diagram.

## 11. Build and validation plan for the future implementation

The `docs/` module includes scripts similar to:

```json
{
  "scripts": {
    "docs:dev": "docusaurus start --host 0.0.0.0",
    "docs:build": "docusaurus build",
    "docs:serve": "docusaurus serve --host 0.0.0.0",
    "docs:typecheck": "tsc --noEmit",
    "docs:lint": "eslint .",
    "docs:check-links": "docusaurus build"
  }
}
```

Expected verification before publishing:

```bash
cd docs
bun install
bun run docs:typecheck
bun run docs:lint
bun run docs:build
bun run docs:serve
```

Smoke checks:

```bash
curl -fsS http://0.0.0.0:3000/docs/
curl -fsS http://0.0.0.0:3000/docs/getting-started/quickstart
curl -fsS http://0.0.0.0:3000/llms.txt
```

## 12. Implementation phases

### Phase A — Scaffold and navigation

Status: **Implemented on 2026-06-08** in `website/`, migrated to `docs/` on 2026-06-10.

- [x] Create Docusaurus app; module path is now `docs/`.
- [x] Configure TypeScript, Bun scripts, i18n, navbar, footer, sidebars, theme CSS.
- [x] Add docs landing page and verified starter P0 page shells for the proposed IA.
- [x] Reuse existing architecture SVG, console tour GIF, and breathing logo GIF.
- [x] Add build validation to local commands.

Acceptance evidence:

- `bun install --frozen-lockfile` passed.
- `bun run docs:typecheck` passed.
- `bun run docs:build` passed and generated English plus `zh-CN` static output.
- `bun run docs:serve -- --port 13030` plus curl smoke passed for `/`, `/docs/`, `/zh-CN/docs/`, `/docs/getting-started/quickstart`, and `/llms.txt`.
- Navbar/sidebar/footer match the planned first scaffold; final deployment domain remains undecided.

### Phase B — First complete P0 docs set

Status: **Implemented on 2026-06-08** for the current P0 sidebar set.

- [x] Enrich Overview, Getting Started, Concepts, SDK, Deployment, Integrations, and Reference P0 pages from verified repository behavior.
- [x] Add SDK pages for Rust, Go, Java Spring Boot, Python, and Node.js.
- [x] Guard English P0 pages with minimum evaluation-depth and section-count checks.
- [x] Keep SDK/runtime claims tied to committed SDK/demo paths and runtime requirements.

Acceptance evidence:

- `python3 .github/tests/docs_site_contract_test.py` passed, including deployment runbook snippets and baseUrl guard.
- `cd docs && bun run docs:typecheck` passed.
- `cd docs && bun run docs:build` passed.
- zh-CN serve smoke passed for P0 routes including installation, Rust, Python, Node.js, Kubernetes, and troubleshooting.
- Default `/tikeo/` and custom root `/` builds were both smoke-tested for zh-CN route switching.

### Phase C — Chinese localization

Status: **Implemented on 2026-06-08** for every current P0 route.

- [x] Provide zh-CN counterparts for all P0 docs routes.
- [x] Add a docs contract requiring real zh-CN localized depth instead of placeholder summaries.
- [x] Keep technical terms consistent with README/Web UI naming where applicable.

Acceptance:

- Language switcher works for every current P0 page.
- No mixed Chinese/English labels except official technology names.

### Phase D — Search, LLM export, and publish readiness

- Add local search or DocSearch.
- Generate `llms.txt` and `llms-full.txt`.
- Add canonical URLs, sitemap, robots, OpenGraph images, and preview cards.
- Add deployment instructions for the chosen external hosting target.

Acceptance:

- Static build produces sitemap and LLM export files.
- Public deployment target is documented but not hardwired to one vendor.

## 13. Initial page priority backlog

P0 for first public docs launch:

1. `/docs/`
2. `/docs/getting-started/installation`
3. `/docs/getting-started/quickstart`
4. `/docs/getting-started/seed-demo-data`
5. `/docs/concepts/worker-tunnel`
6. `/docs/concepts/workflows`
7. `/docs/sdks/rust`
8. `/docs/sdks/go`
9. `/docs/sdks/java-spring-boot`
10. `/docs/sdks/python`
11. `/docs/sdks/nodejs`
12. `/docs/deployment/docker-compose`
13. `/docs/deployment/kubernetes`
14. `/docs/integrations/overview`
15. `/docs/reference/configuration`
16. `/docs/reference/troubleshooting`

P1 after first launch:

- Dynamic scripts governance guide.
- OIDC/RBAC guide.
- Prometheus/Grafana guide.
- Migration from XXL-Job.
- Migration from PowerJob.
- OpenAPI and gRPC reference automation.
- Deeper user-guide pages for Dashboard, Jobs, Instances, Workers, Workflows, Scripts, Audit, and Settings.

P2 after ecosystem expansion:

- SDK overview and cross-language parity guide.
- Terraform Provider guide.
- Kubernetes Operator guide.
- Advanced workflow patterns.
- Large-scale operations runbook.

## 14. Non-goals for the first docs implementation

- Do not build a custom docs framework.
- Do not deploy inside the main Tikeo server binary.
- Do not block on Algolia DocSearch approval.
- Do not create fake package badges or unverified SDK quickstarts.
- Do not generate API reference manually if OpenAPI/protobuf generation can own it later.
- Do not copy the entire architecture design document into one giant docs page.

## 15. Open questions before implementation

- Which final public domain will host the docs site?
- Should locale path be `zh-CN` to match repo files or `zh-Hans` to mirror the reference site?
- Should docs versioning start immediately at `0.x` or wait until the first public release tag?
- Should release notes live in Docusaurus blog or continue only in `CHANGELOG.md` until the first launch?
- Which search option should be used before Algolia DocSearch is available?

## 16. Immediate next action

Phase A scaffold and Phase B P0 content/localization are implemented in `docs/`. Next implementation step: add docs CI or a docs-specific workflow, then expand user-guide/API reference depth from generated OpenAPI/protobuf/source artifacts. Keep deployment provider configuration separate until the final hosting target is chosen.
