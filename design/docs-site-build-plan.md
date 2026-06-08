# Tikeo Docs Site Build Plan

Status: Draft plan only  
Last refreshed: 2026-06-08  
Owner: Tikeo maintainers  
Scope: Future standalone documentation site; this document does **not** scaffold or deploy the site.

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
website/
├─ package.json
├─ bun.lock
├─ docusaurus.config.ts
├─ sidebars.ts
├─ docs/
│  ├─ index.md
│  ├─ overview/
│  ├─ getting-started/
│  ├─ concepts/
│  ├─ user-guide/
│  ├─ sdks/
│  ├─ deployment/
│  ├─ integrations/
│  ├─ guides/
│  ├─ developer-guide/
│  └─ reference/
├─ blog/
│  └─ releases/
├─ src/
│  ├─ css/custom.css
│  ├─ components/
│  └─ pages/index.tsx
├─ static/
│  ├─ img/
│  ├─ video/
│  └─ llms.txt
└─ i18n/
   └─ zh-CN/
      ├─ docusaurus-plugin-content-docs/current/
      ├─ docusaurus-plugin-content-blog/
      └─ code.json
```

Why `website/` instead of `docs/`:

- `docs/` already stores shared assets such as architecture SVGs and the demo GIF.
- A top-level `website/` avoids mixing generated static-site source with existing project docs/assets.
- Docusaurus conventionally uses `website/` or project root; `website/` is cleaner for a Rust workspace.

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

1. Demo GIF or short MP4 loop from `docs/assets/tikeo-console-tour.gif` or the promo video artifact.
2. Four capability cards:
   - Worker Tunnel
   - Workflow DAG Canvas
   - Multi-language SDKs
   - Security and Audit
3. Quickstart command block.
4. Architecture diagram from `docs/assets/tikeo-architecture.en.svg`.
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
| `docs/assets/*` | Architecture diagrams and demo media | Reuse current assets; avoid bloating repository. |

## 8. Documentation quality rules

- Every getting-started page must include exact commands, expected output, and next link.
- Every SDK page must show a minimal worker, registration behavior, execution handler, and verification command.
- Every deployment page must state supported database, required ports, environment variables, and health checks.
- Every security page must distinguish current implemented behavior from roadmap items.
- Do not advertise Python/Node runnable demos until they are actually implemented and verified.
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

When the site is actually scaffolded, add scripts similar to:

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
cd website
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

- Create `website/` Docusaurus app.
- Configure TypeScript, Bun scripts, i18n, navbar, footer, sidebars, theme CSS.
- Add docs landing page and empty page shells for the proposed IA.
- Reuse existing architecture SVG and demo GIF.
- Add build validation to local commands.

Acceptance:

- `bun run docs:build` passes.
- English and Chinese routes render.
- Navbar/sidebar/footer match the planned structure.

### Phase B — First complete English docs set

- Fill Overview, Getting Started, Concepts, User Guide, SDK overview, Deployment overview, and Reference essentials.
- Split `design/tikeo-architecture-design.md` into readable conceptual pages.
- Add code tabs for Rust/Go/Java where verified.
- Keep Python/Node SDK pages marked as planned until runnable.

Acceptance:

- A new evaluator can complete Server + Web + Rust or Go worker quickstart from docs alone.
- No page claims unsupported runtime behavior.

### Phase C — Chinese localization

- Translate high-traffic pages first: Overview, Quickstart, Architecture, SDK overview, Deployment overview, Troubleshooting.
- Then translate all sidebar pages.
- Align terms with `web/src/i18n/messages.ts` and `README.zh-CN.md`.

Acceptance:

- Language switcher works for every first-release page.
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
10. `/docs/deployment/docker-compose`
11. `/docs/deployment/kubernetes`
12. `/docs/reference/configuration`
13. `/docs/reference/troubleshooting`

P1 after first launch:

- Dynamic scripts governance guide.
- OIDC/RBAC guide.
- Prometheus/Grafana guide.
- Migration from XXL-Job.
- Migration from PowerJob.
- OpenAPI and gRPC reference automation.
- Full Chinese localization.

P2 after ecosystem expansion:

- Python SDK page.
- Node.js SDK page.
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

When the user approves implementation, create `website/` as a Docusaurus 3 docs app with the IA above, add only verified starter content, and wire build validation. Keep deployment provider configuration separate until the user chooses the final hosting target.
