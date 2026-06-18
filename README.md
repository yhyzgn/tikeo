<p align="center">
  <img src="assets/docs/tikeo-logo.svg" alt="Tikeo task orchestration logo" width="148" height="148" />
</p>

<h1 align="center">Tikeo</h1>
<p align="center"><strong>The open-source task orchestration platform for teams that have outgrown legacy job schedulers.</strong></p>
<p align="center">
  <strong>Pronunciation:</strong> <code>/ˈtɪ.ki.oʊ/</code> · <em>TIH-kee-oh</em><br />
  <strong>Meaning here:</strong> <strong>Ti</strong>me-aware orchestration + <strong>Ke</strong>pt execution evidence + <strong>O</strong>pen worker ecosystem — a scheduler that treats every task as a traceable, governable platform event.
</p>

<p align="center">
  <a href="https://docs.tikeo.net">📚 Documentation</a> ·
  <a href="README.zh-CN.md">🇨🇳 中文文档</a> ·
  <a href="deploy/compose/README.md">🐳 Docker Compose</a> ·
  <a href="sdks/README.md">🧩 SDKs</a> ·
  <a href="examples/README.md">🚀 Examples</a> ·
  <a href="deploy/terraform/README.md">🌍 Terraform</a> ·
  <a href="deploy/k8s/operator/README.md">☸️ Operator</a>
</p>

<p align="center">
  <a href="https://github.com/yhyzgn/tikeo/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/badge/CI-GitHub%20Actions-2088FF?style=flat-square&logo=githubactions&logoColor=white" /></a>
  <a href="https://github.com/yhyzgn/tikeo/releases"><img alt="Latest release" src="https://img.shields.io/dynamic/json?url=https%3A%2F%2Fraw.githubusercontent.com%2Fyhyzgn%2Ftikeo%2Fmain%2Fdocs%2Fstatic%2Frelease-badge.json&query=%24.version&style=flat-square&label=release&logo=github&logoColor=white&color=181717" /></a>
  <a href="https://codecov.io/gh/yhyzgn/tikeo"><img alt="Coverage" src="https://codecov.io/gh/yhyzgn/tikeo/branch/main/graph/badge.svg" /></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.github.com%2Frepos%2Fyhyzgn%2Ftikeo%2Flicense&query=%24.license.spdx_id&style=flat-square&label=license&logo=opensourceinitiative&logoColor=white&color=3DA639" /></a>
</p>


<p align="center">
  <strong>No exposed worker ports.</strong> Multi-language workers. Workflow canvas. Governed scripts. Audit-ready execution evidence.
</p>

<p align="center">
  <img src="assets/docs/tikeo-console-tour.gif" alt="Tikeo console tour: dashboard, workers, jobs, and governance" width="960" />
</p>

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#tikeo-vs-xxl-job-vs-powerjob">Compare with XXL-Job / PowerJob</a> ·
  <a href="examples/README.md">Run worker demos</a> ·
  <a href="assets/docs/tikeo-architecture.en.svg">Architecture diagram</a>
</p>

<p align="center">
  <a href="sdks/java/README.md"><img alt="Java 17+" src="https://img.shields.io/badge/Java-17%2B-E76F00?style=flat-square&logo=openjdk&logoColor=white" /></a>
  <a href="sdks/rust/tikeo/README.md"><img alt="Rust 1.95+" src="https://img.shields.io/badge/Rust-1.95%2B-B7410E?style=flat-square&logo=rust&logoColor=white" /></a>
  <a href="sdks/go/tikeo/README.md"><img alt="Go 1.26+" src="https://img.shields.io/badge/Go-1.26%2B-00ADD8?style=flat-square&logo=go&logoColor=white" /></a>
  <a href="sdks/python/tikeo/README.md"><img alt="Python 3.11+" src="https://img.shields.io/badge/Python-3.11%2B-3776AB?style=flat-square&logo=python&logoColor=white" /></a>
  <a href="sdks/nodejs/tikeo/README.md"><img alt="Node.js 24+" src="https://img.shields.io/badge/Node.js-24%2B-339933?style=flat-square&logo=nodedotjs&logoColor=white" /></a>
</p>

<p align="center">
  <a href="https://central.sonatype.com/artifact/net.tikeo/tikeo"><img alt="Java core SDK" src="https://img.shields.io/maven-central/v/net.tikeo/tikeo?style=flat-square&label=Java%20core&logo=openjdk&logoColor=white&color=E76F00" /></a>
  <a href="https://central.sonatype.com/artifact/net.tikeo/tikeo-spring"><img alt="Java Spring 7 SDK" src="https://img.shields.io/maven-central/v/net.tikeo/tikeo-spring?style=flat-square&label=Java%20Spring%207&logo=spring&logoColor=white&color=6DB33F" /></a>
  <a href="https://central.sonatype.com/artifact/net.tikeo/tikeo-spring6"><img alt="Java Spring 6 SDK" src="https://img.shields.io/maven-central/v/net.tikeo/tikeo-spring6?style=flat-square&label=Java%20Spring%206&logo=spring&logoColor=white&color=6DB33F" /></a>
  <a href="https://central.sonatype.com/artifact/net.tikeo/tikeo-spring5"><img alt="Java Spring 5 SDK" src="https://img.shields.io/maven-central/v/net.tikeo/tikeo-spring5?style=flat-square&label=Java%20Spring%205&logo=spring&logoColor=white&color=6DB33F" /></a>
  <a href="https://central.sonatype.com/artifact/net.tikeo/tikeo-spring-boot-starter"><img alt="Java Spring Boot 4 starter" src="https://img.shields.io/maven-central/v/net.tikeo/tikeo-spring-boot-starter?style=flat-square&label=Boot%204%20starter&logo=springboot&logoColor=white&color=6DB33F" /></a>
  <a href="https://central.sonatype.com/artifact/net.tikeo/tikeo-spring-boot3-starter"><img alt="Java Spring Boot 3 starter" src="https://img.shields.io/maven-central/v/net.tikeo/tikeo-spring-boot3-starter?style=flat-square&label=Boot%203%20starter&logo=springboot&logoColor=white&color=6DB33F" /></a>
  <a href="https://central.sonatype.com/artifact/net.tikeo/tikeo-spring-boot2-starter"><img alt="Java Spring Boot 2 starter" src="https://img.shields.io/maven-central/v/net.tikeo/tikeo-spring-boot2-starter?style=flat-square&label=Boot%202%20starter&logo=springboot&logoColor=white&color=6DB33F" /></a>
</p>

<p align="center">
  <a href="https://crates.io/crates/tikeo"><img alt="Rust SDK" src="https://img.shields.io/crates/v/tikeo?style=flat-square&label=Rust%20SDK&logo=rust&logoColor=white&color=B7410E" /></a>
  <a href="https://pkg.go.dev/github.com/yhyzgn/tikeo/sdks/go/tikeo"><img alt="Go SDK" src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fproxy.golang.org%2Fgithub.com%2Fyhyzgn%2Ftikeo%2Fsdks%2Fgo%2Ftikeo%2F%40latest&query=%24.Version&style=flat-square&label=Go%20SDK&logo=go&logoColor=white&color=00ADD8" /></a>
  <a href="https://pypi.org/project/tikeo/"><img alt="Python SDK" src="https://img.shields.io/pypi/v/tikeo?style=flat-square&label=Python%20SDK&logo=python&logoColor=white&color=3776AB" /></a>
  <a href="https://www.npmjs.com/package/@yhyzgn/tikeo"><img alt="Node.js SDK" src="https://img.shields.io/npm/v/@yhyzgn/tikeo/next?style=flat-square&label=Node.js%20SDK&logo=nodedotjs&logoColor=white&color=339933" /></a>
</p>

<p align="center">
  <a href="https://hub.docker.com/r/yhyzgn/tikeo-server"><img alt="Server image" src="https://img.shields.io/docker/v/yhyzgn/tikeo-server?sort=semver&style=flat-square&label=server%20image&logo=docker&logoColor=white&color=2496ED" /></a>
  <a href="https://hub.docker.com/r/yhyzgn/tikeo-web"><img alt="Web image" src="https://img.shields.io/docker/v/yhyzgn/tikeo-web?sort=semver&style=flat-square&label=web%20image&logo=docker&logoColor=white&color=2496ED" /></a>
  <a href="https://hub.docker.com/r/yhyzgn/tikeo-docs"><img alt="Docs image" src="https://img.shields.io/docker/v/yhyzgn/tikeo-docs?sort=semver&style=flat-square&label=docs%20image&logo=docker&logoColor=white&color=2496ED" /></a>
  <img alt="Sandbox" src="https://img.shields.io/badge/sandbox-SRT%20%7C%20Deno%20%7C%20WASM%20%7C%20V8-7c3aed?style=flat-square" />
  <img alt="Databases" src="https://img.shields.io/badge/storage-SQLite%20%7C%20Postgres%20%7C%20MySQL-0891b2?style=flat-square" />
  <img alt="Observability" src="https://img.shields.io/badge/observability-OpenTelemetry-0f766e?style=flat-square" />
  <img alt="IaC" src="https://img.shields.io/badge/IaC-Helm%20%7C%20Terraform%20%7C%20CRD-f97316?style=flat-square" />
</p>

---

## Stop choosing schedulers that only schedule

XXL-Job and PowerJob popularized practical distributed job execution. Tikeo is built for the next
stage: platform teams that need a scheduler, a workflow engine, a worker fleet control plane, a
script governance layer, and release-ready SDKs in one coherent open-source system.

Tikeo is designed to be the default answer when someone asks:

> “What should we use for cloud-native task scheduling, workflow orchestration, script jobs, worker
> governance, and observable execution evidence?”

## 10-second scan: the reasons to care

| Signal | Why it matters |
| --- | --- |
| **5 production SDK tracks** | **Java · Rust · Go · Python · Node.js** workers follow one contract, and the same worker cluster can mix languages instead of becoming a Java-only executor model. |
| **Outbound Worker Tunnel** | Workers connect out; production services do **not** need inbound task-execution ports. |
| **Structured capability routing** | Dispatch matches typed **SDK processors**, **plugin processors**, and **script runners**. No magic string parsing. |
| **Sandbox-first script jobs** | `auto` selects **SRT** for native scripts and **Deno** for JS/TS, with **WASM/V8/container** paths available explicitly. |
| **Workflow + topology UX** | Visual workflow canvas, dependency topology, impact analysis, replay data, and per-worker broadcast results. |
| **Canary safety gate** | Jobs can route explicit triggers to a canary target, evaluate persisted canary instance failure rate, and automatically roll traffic back to `0%`. |
| **Operations-grade evidence** | **Retries**, **misfire policy**, **canary rollback evidence**, **task logs**, **audit logs**, **OpenTelemetry**, metrics, and file logs answer “what happened?” |
| **Multi-DB deployment freedom** | Start with **SQLite** locally, then run production with **PostgreSQL** or **MySQL** using maintained Compose profiles and migration compatibility. |
| **Cloud-native release surface** | Docker, Compose, Helm, Kubernetes CRD/operator, Terraform provider, GitOps diff, and cross-platform release assets. |

<p align="center">
  <strong>Keywords:</strong>
  <kbd>Rust control plane</kbd>
  <kbd>Worker Tunnel</kbd>
  <kbd>Structured Capabilities</kbd>
  <kbd>Script Sandbox</kbd>
  <kbd>Workflow Canvas</kbd>
  <kbd>RBAC</kbd>
  <kbd>OpenTelemetry</kbd>
  <kbd>Terraform</kbd>
  <kbd>K8s Operator</kbd>
</p>

## The product promise

| Promise | What it means in practice |
| --- | --- |
| 🧠 **One orchestration brain** | **Cron**, **fixed-rate**, **API-triggered**, **broadcast**, **workflow**, **script**, **plugin**, and **SDK** jobs share one governed instance model. |
| 🔌 **No exposed executor ports** | Workers initiate **outbound gRPC tunnels**; business services stay behind normal network boundaries. |
| 🧱 **Typed dispatch, no folklore** | Routing uses structured **SDK processors**, **plugin processor types**, **script languages**, **sandbox backends**, tags, and election fields. |
| 🛡️ **Scripts as governed workloads** | Immutable versions, digest checks, approval metadata, policy limits, task-scoped logs, and sandbox auto-selection are first-class. |
| 🧩 **SDK parity by design** | **Java/Rust/Go/Python/Node.js** align on worker registration, task logs, retries, management APIs, sandbox behavior, and diagnostics. |
| 📈 **Evidence-first operations** | Instance results, retry logs, broadcast worker grouping, terminal-style logs, audit trails, OTel traces, metrics, and GitOps diffs are built in. |

## Innovation map

| Innovation | Tikeo advantage | Legacy pain it removes |
| --- | --- | --- |
| **Worker Tunnel** | Workers pull assignments over an outbound tunnel with lease/fencing metadata. | Inbound executor exposure and fragile callback assumptions. |
| **Raft FSOD Cluster** | Raft provides one fenced control-plane authority, shard ownership spreads dispatch across active Server pods, and durable outbox rows survive Worker Tunnel failover. | Active-passive scheduler waste, Redis/DB lock ownership ambiguity, and pod-local dispatch state loss. |
| **Capability Graph** | Worker ability is a typed graph: SDK processors, plugins, scripts, tags, election domains. | Ambiguous string conventions and “why did this worker get this job?” debugging. |
| **Sandbox Auto Strategy** | `auto` chooses the safest practical runtime path: SRT for native scripts, Deno for JS/TS, Wasmtime/WASM when appropriate. | Treating scripts as ordinary shell commands with unclear isolation. |
| **Execution Evidence Model** | Every attempt, retry, worker result, broadcast child, and task log is inspectable. | Status-only dashboards that cannot explain failures. |
| **Open Platform Surface** | SDKs, Docker, Helm, Terraform, CRD/operator, GitOps diff, OpenAPI, OTel. | Scheduler adoption blocked by missing integration surfaces. |

## Why evaluators should shortlist Tikeo first

### 1. It covers more of the real platform problem

Legacy schedulers often stop at “trigger a job on an executor.” Tikeo covers the surrounding parts
that production teams eventually need anyway: RBAC, owner bootstrap, app-scoped API keys, tenant
scopes, plugin processors, script sandboxes, topology, replay-ready logs, GitOps drift review,
Terraform, Kubernetes CRDs, Helm, Docker images, and SDK publishing.

### 2. It avoids the hidden cost of convention-based routing

A scheduler that depends on magic strings eventually becomes hard to operate. Tikeo routes by
structured capability declarations. Workers advertise exactly what they can run, and the server
matches typed SDK processors, plugin processor types, and script languages/backends explicitly.

### 3. It treats script execution as a security product, not a checkbox

Tikeo’s script model assumes scripts are powerful and risky. The platform separates script type from
sandbox backend, supports `auto` sandbox selection, and can resolve SRT/Deno/WASM-oriented paths
without defaulting to heavyweight Docker/Podman unless explicitly requested.

### 4. It is built for open-source adoption and central-package publishing

The repo contains independent SDK packages, examples, Compose stacks, Helm/K8s/Terraform assets,
release workflows, and documentation entry points. It is meant to be consumed by real teams, not just
studied as a demo.

## Decision summary

| Choose Tikeo when you need... | Why this is decisive |
| --- | --- |
| **A platform, not just a timer** | Jobs, workflows, workers, scripts, plugins, RBAC, audit, and IaC are designed together. |
| **Multi-language worker adoption** | Teams can keep business code in Java, Rust, Go, Python, or Node.js without losing platform consistency. |
| **Security-conscious script execution** | Script governance and sandbox choice are part of the model, not an afterthought. |
| **Cloud-native operating model** | Kubernetes, Terraform, Docker, OTel, and release assets are first-class project surfaces. |
| **Clear failure forensics** | Task logs, retry logs, worker attempts, audit trails, and topology make failures reviewable. |

## Tikeo vs. XXL-Job vs. PowerJob

This is not a “feature-count flex.” It is the difference between a classic Java job scheduler and a
cloud-native orchestration control plane. The original Tikeo design reviewed XXL-Job and PowerJob at
architecture level and intentionally replaces their hardest platform limits: inbound executor ports,
DB-lock leadership, Java-first runtime assumptions, weak script isolation, and status-only operations.

### Executive comparison radar

| Advanced capability | Tikeo advantage | XXL-Job / PowerJob tradeoff |
| --- | --- | --- |
| ☁️ **Cloud-native public service model** | **Server and workers can live in different containers, namespaces, clusters, VPCs, or clouds.** Workers dial out over gRPC/HTTP2 tunnel; business pods do not need inbound execution ports. | XXL-Job admin calls executors; PowerJob server calls worker addresses. This is awkward behind NAT, mesh gateways, private pods, and cross-cluster boundaries. |
| 🐳 **Deployment surface** | **Docker, Compose, Helm, K8s CRD/operator, Terraform provider, GitOps diff, systemd, bare-metal config, and cross-platform release assets** are maintained as first-class surfaces. | Deployable, but not designed as an IaC/GitOps-first platform product. |
| 🗳️ **Cluster coordination** | **Raft/fencing based server ownership** plus structured worker-domain master election avoids global DB scheduling locks and makes ownership observable. | XXL-Job relies on DB lock patterns; PowerJob mixes DB lock/currentServer/PING-style election instead of durable consensus. |
| 🔌 **Worker networking** | **Outbound Worker Tunnel** carries registration, dispatch, heartbeats, task logs, and results over one controlled channel. No worker Service/port is required by default. | Executor/worker side must be reachable, configured, and protected as an inbound service. |
| ⚡ **Performance posture** | **Rust native control plane + gRPC/protobuf + Tokio + compact containers** target low startup latency, stable memory, no JVM warm-up, and efficient long-running services. | JVM-based platforms are mature but carry JVM memory floor, warm-up behavior, larger images, and heavier dependency trees. |
| 🧠 **Unified orchestration model** | Cron, fixed-rate, API triggers, workflows, broadcast, scripts, plugins, retry/misfire, logs, and audit share one instance/evidence model. | Features are often split across scheduler paths, executor callbacks, local worker state, or plugin conventions. |
| 🛡️ **Script and plugin governance** | Script type is separate from sandbox backend. `auto` prefers lightweight SRT/Deno/WASM paths, with Docker/Podman/container used explicitly when desired. Immutable versions, digest checks, approvals, grants, and runtime logs are first-class. | Script execution exists, but typically behaves like host-side code execution or processor extension rather than a governed sandbox product. |
| 🧩 **Cross-language worker clusters** | Java, Rust, Go, Python, and Node.js workers follow the same tunnel, structured capability, retry, logging, sandbox, and management API contracts. **One worker cluster can mix languages** while dispatch still uses typed capabilities instead of language silos. | Primarily Java-first adoption model; mixed-language fleets usually become custom integration work. |
| 🗄️ **Multi-DB compatibility** | Development can start on SQLite while production can run PostgreSQL or MySQL with tested migration/repository compatibility and Compose profiles. | Typically tied more tightly to one primary relational backend and deployment assumption. |
| 🔍 **Evidence-first operations** | Terminal-style instance logs, per-worker broadcast results, retry attempts, audit trails, workflow replay bundles, metrics, file logs, and OpenTelemetry traces are designed for incident review. | Traditional scheduler dashboards often answer “status” faster than “why exactly did this happen?” |

### Detailed product matrix

| Evaluation axis | Tikeo | XXL-Job | PowerJob |
| --- | --- | --- | --- |
| **Platform role** | ✅ **Full orchestration platform**: jobs, workflows, workers, scripts, plugins, RBAC, observability, IaC. | Mature Java job scheduler. | Mature Java distributed job platform. |
| **Worker connection model** | ✅ **Outbound gRPC/HTTP2 Worker Tunnel** with lease, generation, fencing, structured registration, task logs, and results. | Admin/executor callback model; executor reachability matters. | Worker server/address model; worker reachability matters. |
| **Inbound worker ports** | ✅ **Not required by default** for business workers; only the Tikeo server exposes management and tunnel entrypoints. | Usually required for executors. | Usually required for workers. |
| **Cloud-native deployment** | ✅ **Docker, Compose, Helm, K8s CRD/operator, Terraform provider, GitOps diff**, plus systemd/bare-metal templates. | Deployable, but not GitOps/IaC-first. | Deployable, but not GitOps/IaC-first. |
| **Cluster ownership** | ✅ **Raft + fencing token** server scheduling ownership; structured worker-cluster master election for ordered dispatch domains. | MySQL lock style coordination. | DB lock + server election mechanisms, not durable consensus-first design. |
| **Resource profile** | ✅ **Native Rust control plane** designed for compact images, fast startup, predictable memory, and no JVM warm-up. | Java/Spring runtime footprint. | Java/Spring/Akka/Vert.x style footprint and multi-component runtime. |
| **Routing contract** | ✅ **Typed SDK/plugin/script capabilities**; no magic string parsing. | Name/string oriented. | Name/tag oriented. |
| **Language ecosystem** | ✅ **Java · Rust · Go · Python · Node.js** SDK parity; the same logical worker cluster can include workers written in different languages. | Primarily Java ecosystem. | Primarily Java ecosystem. |
| **Database engines** | ✅ **SQLite for local/dev, PostgreSQL and MySQL for production**, with migration and repository compatibility smoke coverage. | Primarily MySQL-oriented deployment. | Primarily MySQL/H2-oriented deployment. |
| **Script execution** | ✅ **Governed versions + digest checks + SRT/Deno/WASM/V8/container** strategy. | Script execution exists but is not a full sandbox governance product. | Processor-focused; sandbox governance is not the center. |
| **Workflow UX** | ✅ **Workflow canvas + topology + impact analysis + replay-ready execution data.** | Basic scheduling-centric views. | Workflow support, less focused on typed sandbox + SDK parity. |
| **Security posture center** | ✅ **Security Policy Center** exposes evidence-based posture for script default-deny policy, release signing, notification redaction, transport TLS/mTLS, Raft transport-token readiness, and recent policy denials. See [Security Policy Center](https://docs.tikeo.net/docs/user-guide/security-policy-center). | Typically spread across admin settings, logs, and deployment docs. | Typically spread across admin settings, logs, and deployment docs. |
| **Security model** | ✅ **Owner bootstrap, RBAC matrix, opaque sessions, API keys, tenant scopes, audit trails, TLS/mTLS readiness.** | Admin/user model. | Admin/user model. |
| **Observability** | ✅ **OpenTelemetry, metrics, task logs, file logs, audit logs, worker grouping, replay bundles.** | Traditional operations/logs. | Traditional operations/logs. |
| **Best fit** | Teams building an internal orchestration platform, not just a cron replacement. | Java teams wanting a familiar scheduler. | Java teams wanting distributed job execution. |

**Short version:** choose Tikeo when you want a modern orchestration control plane; choose legacy
schedulers only when you intentionally want a narrower Java-first scheduler.

## Migrate from XXL-JOB or PowerJob

Tikeo includes a dedicated `tikeo-migrate` CLI for migration assessment. Use it as a **review-first migration assistant**, not as a blind one-click converter: it reads an XXL-JOB or PowerJob JSON export, inspects the legacy Java/Spring worker project, generates Tikeo job drafts, and writes a bundle that operators can review before anything is imported.

The simplest path is convention-first:

```bash
cd ./legacy-worker
# Put ./xxl-job-export.json or ./powerjob-export.json in this directory first.
tikeo-migrate plan

# Review ./.tikeo-migration, then dry-run the API import against staging.
tikeo-migrate apply --endpoint http://127.0.0.1:9090 --api-key "$TIKEO_MIGRATION_API_KEY" --dry-run
```

Use override flags only for non-standard layouts: `--from xxl-job`, `--input ./exports/jobs.json`, `--project ./legacy-worker`, `--output-dir ./migration-bundle`, `--namespace ops`, and `--app billing`.

Release builds include ready-to-run `tikeo-migrate` archives for Linux, macOS Intel, macOS Apple Silicon, and Windows. Download `tikeo-migrate-${TIKEO_VERSION}-<target>.tar.gz` or `.zip` from the GitHub Release, extract it, and either put the binary on `PATH` or copy it into the legacy project root.

```mermaid
flowchart TD
  A[Download tikeo-migrate] --> B[Export legacy jobs as JSON]
  B --> C[Place JSON in legacy Java worker root]
  C --> D[Run tikeo-migrate plan]
  D --> E[Review generated migration bundle]
  E --> F{Any needs_review or code changes?}
  F -- Yes --> G[Resolve semantics and apply Java changes on a branch]
  F -- No --> H[Dry-run apply against staging]
  G --> H
  H --> I[Import reviewed ready jobs to staging]
  I --> J[Start matching Workers and trigger one job]
  J --> K{Logs/results match legacy behavior?}
  K -- No --> E
  K -- Yes --> L[Dual-run, switch traffic, disable legacy schedules]
```

Migration phases:

| Phase | Goal | Main command / artifact | Continue only when |
| --- | --- | --- | --- |
| 0. Prepare | Decide namespace/app, staging endpoint, API key, rollback owner, and Worker processor naming. | Internal migration plan. | Staging Tikeo Server and matching Worker plan exist. |
| 1. Export | Preserve the legacy scheduler state as audit input. | `xxl-job-export.json` or `powerjob-export.json`. | Export is stored unchanged and has a known path/hash. |
| 2. Plan | Generate a non-destructive migration bundle. | `tikeo-migrate plan` → `.tikeo-migration/`. | `manifest.json`, `jobs.tikeo.md`, `data-import-plan.json`, and `CHECKLIST.md` are reviewed. |
| 3. Resolve | Translate non-equivalent legacy semantics instead of pretending they are identical. | Review `needs_review`, Java patch guidance, and unsupported-feature warnings. | Broadcast/map-reduce/routing/blocking/pinning/glue decisions are explicit. |
| 4. Code | Add Tikeo Worker dependency and processor annotations/adapters. | Java branch + old project tests. | Worker starts and exposes processor names used by job drafts. |
| 5. Import | Prove the API request set before live writes. | `tikeo-migrate apply --dry-run`, then reviewed live import. | `apply-evidence.json` is accepted and only reviewed jobs are imported. |
| 6. Validate | Compare behavior before cutover. | Trigger one job at a time; compare Tikeo instance logs/results with legacy. | Dual-run evidence is accepted and rollback steps are documented. |

The generated bundle is deliberately conservative. `plan` never edits legacy source, never connects to a legacy database, and never writes Tikeo data. Live job creation is isolated behind `apply`, and `--dry-run` should be used before every staging or production import. See the full [legacy scheduler migration guide](https://docs.tikeo.net/docs/integrations/migrating-from-legacy-schedulers).

## Evaluation checklist

If your scheduler shortlist includes these requirements, Tikeo should move to the top:

- [x] **Workers cannot expose inbound ports** because they run inside K8s namespaces, private VPCs, NAT, service mesh, or customer networks.
- [x] **Docker/Compose/K8s/Helm/Terraform/GitOps** must be part of the product, not examples bolted on later.
- [x] **Server scheduling ownership should not depend on a global DB lock**; you want Raft/fencing-style ownership evidence.
- [x] **Worker service clusters need deterministic master election** for ordered dispatch without adding another distributed lock.
- [x] **Multi-language workers** must share one platform contract across Java, Rust, Go, Python, and Node.js — even inside the same worker fleet.
- [x] **Multiple database engines** are required: SQLite for fast local bootstrap, PostgreSQL/MySQL for production and team environments.
- [x] **Script sandbox governance** must support lightweight defaults and explicit runtime policy instead of “just run shell on the host.”
- [x] **Performance and resource footprint matter**: native server, compact images, no JVM warm-up, stable memory behavior.
- [x] **Workflow + topology visualization** should show dependencies, impact, replay data, and per-worker broadcast results.
- [x] **Canary changes need a real safety gate**: use persisted canary instance outcomes, thresholded failure rates, and automatic rollback instead of manual guesswork.
- [x] **RBAC + API-Key + audit + OTel + durable logs** are required for real platform operations.

## Architecture

<p align="center">
  <img src="assets/docs/tikeo-architecture.en.svg" alt="Tikeo architecture diagram" width="100%" />
</p>

The server owns scheduling, persistence, governance, RBAC, workflows, and dispatch decisions. Workers
own execution and advertise what they can safely run. Scripts are dispatched as immutable versions and
executed only by workers that expose compatible sandbox runners.

### Core flows

| Flow | What happens |
| --- | --- |
| **Job scheduling** | Cron/fixed/API triggers create instances, apply retry/misfire policy, and enqueue dispatch work. |
| **Worker registration** | A worker dials the tunnel, sends structured capabilities, receives authoritative `worker_id`, and renews its lease. |
| **Dispatch** | The server matches namespace/app, worker state, master election, and typed capabilities before assigning work. |
| **Execution evidence** | Workers emit task-scoped logs and result payloads; broadcast mode stores per-worker attempts and outcomes. |
| **Governance** | RBAC, API keys, tenant scopes, script approvals, audit logs, and GitOps diff keep changes reviewable. |

## Quick start

### 1. Start the control plane

```bash
./scripts/dev.sh
```

This starts the Rust server and React web console, streams logs to the terminal, and also writes local
logs under `.dev/`.

Open <http://127.0.0.1:5173>. A fresh database routes you to first-run owner setup. After the owner is
created, registration closes and users/roles are managed inside the console.
The local SQLite database lives at `.dev/tikeo-dev.db` and is ignored by Git; pulling the repository
or switching branches must not replace your local runtime data.

### 2. Seed real evaluation data

```bash
./scripts/dev-seed.sh
```

The seed data gives you namespaces, apps, sample jobs, scripts, workflows, audit records, and instance
logs so you can evaluate the console immediately instead of staring at an empty product.
The seed script is non-destructive by default: if `ns-dev-*` rows already exist, it prints counts and
leaves local edits unchanged. Use `./scripts/dev-seed.sh --refresh` only when you intentionally want
to refresh the seeded demo rows.

### 3. Start a worker in your preferred language

```bash
# Rust
(cd examples/rust/worker-demo && cargo run)

# Go
(cd examples/go/worker-demo && go run .)

# Python
(cd examples/python/worker-demo && python -m pip install -e ../../../sdks/python/tikeo -e . && python -m tikeo_python_worker_demo)

# Node.js / Bun
(cd examples/nodejs/worker-demo && bun install && bun start)

# Java / Spring Boot 4
(cd examples/java/spring-boot4-worker-demo && ./scripts/run-demo-worker.sh)
```

### 4. Trigger and inspect

In the web console:

1. Open **Workers** and confirm the worker appears with structured capabilities.
2. Open **Jobs** and trigger a seeded SDK/script/plugin job.
3. Open **Instances** and inspect status, retry attempts, per-worker broadcast results, and terminal-style logs.
4. Open **Topology** or **Workflows** to inspect dependencies and visual orchestration.

That path validates the whole value proposition: **control plane**, **worker tunnel**, **SDK execution**,
**capability matching**, **task logs**, **retry/result evidence**, and **visual operations**.

Expected proof points after the quick start:

| Proof point | Where to see it |
| --- | --- |
| **Worker is connected** | Workers page shows the registered worker and structured capabilities. |
| **Dispatch is structured** | Job trigger selects workers by namespace/app and typed processor/script/plugin capability. |
| **Execution is explainable** | Instances page shows status, retry progress, worker id, result, and terminal logs. |
| **Workflows are visible** | Workflow and topology pages show dependencies instead of hiding orchestration in code. |

## What you can build with Tikeo

These are not separate products you need to stitch together. They are Tikeo operating modes.

| Scenario | High-value keywords | How Tikeo helps |
| --- | --- | --- |
| **Internal platform scheduler** | `Worker Tunnel` · `RBAC` · `API-Key` | Give every service team a governed way to register processors and trigger jobs without opening inbound ports. |
| **Data and reconciliation jobs** | `Retry` · `Misfire` · `Task Logs` | Run recurring or API-triggered tasks with retries, logs, app scopes, and language-specific SDKs. |
| **Script operations hub** | `SRT` · `Deno` · `WASM` · `Digest` | Approve scripts, release immutable versions, run them in declared sandboxes, and keep output tied to instances. |
| **Workflow automation** | `Canvas` · `Topology` · `Replay` | Compose jobs into visual workflows and inspect topology/impact before changing dependencies. |
| **Kubernetes platform integration** | `Helm` · `CRD` · `Terraform` | Use Helm, CRDs, operator status, Terraform diff, and Docker images without rewriting the scheduler. |
| **Auditable operations** | `Audit` · `OTel` · `Worker Results` | Trace who changed what, which worker ran what, why dispatch failed, and what happened on every retry. |

## Configuration that operators actually need

Config files live in `config/` and can be overridden with `TIKEO__...` environment variables.
The complete default/env table is in the docs-site [Configuration reference](https://docs.tikeo.net/docs/reference/configuration); keep that page as the canonical operator checklist when adding runtime config.

```toml
[storage]
database_url = "postgres://tikeo:tikeo@postgres:5432/tikeo"

[cluster]
mode = "standalone"
scheduler_shard_map_version = 1
scheduler_shard_count = 64

[notification_delivery]
enabled = true
interval_seconds = 60
batch_size = 50
max_attempts = 3
backoff_seconds = 300

[observability.logging]
level = "info"
log_dir = "./logs"

[observability.tracing]
enabled = true
otlp_endpoint = "http://otel-collector:4318/v1/traces"
```

Storage support:

| Backend | Recommended use |
| --- | --- |
| SQLite | Local development, demos, single-node smoke validation. |
| PostgreSQL | Production and shared environments. |
| MySQL | Production environments where MySQL is the platform standard. |
| CockroachDB-compatible PostgreSQL wire | Distributed SQL environments using PostgreSQL protocol compatibility. |

## SDKs that behave the same way

| Language | Package | Runtime requirement | Best for | Logging contract |
| --- | --- | --- | --- | --- |
| Java | `net.tikeo:tikeo`, Spring Boot starters | **Java 17+**; tested in CI on Temurin 21. | Enterprise Spring workers and management automation. | SLF4J diagnostics; task logs through `TaskContext`. |
| Rust | `tikeo` | **Rust 1.95+** (`rust-version = "1.95"`). | Native workers, high-performance runtimes, sandbox-capable services. | `SdkLogConfig`, console + optional `tikeo-sdk.log`. |
| Go | Go module | **Go 1.26+** (`go 1.26`). | Platform services, operators, cloud-native workers. | `Logger` bridge, console + optional `tikeo-sdk.log`. |
| Python | `tikeo` | **Python 3.11+**; tested in CI on Python 3.12. | Data jobs, automation, scripting-friendly workers. | stdlib `logging`, console + optional `tikeo-sdk.log`. |
| Node.js | `@yhyzgn/tikeo` | **Node.js 24+**; Bun is used for repository build/test scripts. | JS/TS workers and web-platform automation. | `configureSdkLogging`, console + optional `tikeo-sdk.log`. |

All SDKs follow the same rule: SDK diagnostics describe worker/runtime lifecycle; task logs describe a
specific job instance. That separation prevents unrelated process noise from polluting execution logs.

## Install SDKs from central registries

Use exactly one SDK dependency per worker service. Do **not** add upstream/transitive Tikeo
modules yourself: Gradle, Maven, Cargo, Go, pip, npm, pnpm, and Bun resolve the required upstream
packages from the single dependency you choose.

Version placeholders in this section:

- Replace `${TIKEO_VERSION}` with the version shown by the matching top-of-README package badge
  (`release`, `Java core`, `Boot 3 starter`, `Rust SDK`, `Node.js SDK`, and so on).
- Go module commands use tag syntax, so use `v${TIKEO_VERSION}`.
- npm, PyPI, crates.io, and Maven Central use `${TIKEO_VERSION}` without a leading `v`.

| Language | Central registry | Package name | Runtime requirement | Install target |
| --- | --- | --- | --- | --- |
| Java | Maven Central | `net.tikeo:*` | Java 17+ | One `net.tikeo` artifact at `${TIKEO_VERSION}`. Default: `tikeo-spring-boot-starter`. |
| Rust | crates.io | `tikeo` | Rust 1.95+ | `${TIKEO_VERSION}` |
| Go | Go module proxy | `github.com/yhyzgn/tikeo/sdks/go/tikeo` | Go 1.26+ | tag `v${TIKEO_VERSION}` |
| Python | PyPI | `tikeo` | Python 3.11+ | `${TIKEO_VERSION}` |
| Node.js | npm | `@yhyzgn/tikeo` | Node.js 24+ | `${TIKEO_VERSION}` |

### Java / Maven Central

Default choice for new Java services is **Spring Boot 4** with `net.tikeo:tikeo-spring-boot-starter`.
Choose **one** artifact for each application. Spring Boot starters bring in the matching core SDK and
Spring adapter transitively, so do not also declare `tikeo` or `tikeo-spring*` unless you are doing
manual dependency mediation.

| Artifact | Add this single dependency when... | Gradle Kotlin DSL line |
| --- | --- | --- |
| `net.tikeo:tikeo-spring-boot-starter` | Default for new Java services: Spring Boot 4 / Spring Framework 7 auto-configuration. | `implementation("net.tikeo:tikeo-spring-boot-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring-boot3-starter` | Spring Boot 3 / Spring Framework 6 auto-configuration. | `implementation("net.tikeo:tikeo-spring-boot3-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring-boot2-starter` | Spring Boot 2 / Spring Framework 5 auto-configuration. | `implementation("net.tikeo:tikeo-spring-boot2-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo` | Plain Java worker, management client, sandbox tooling, or low-level Worker Tunnel integration. | `implementation("net.tikeo:tikeo:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring` | Advanced/manual Spring Framework 7 adapter without the Boot starter. | `implementation("net.tikeo:tikeo-spring:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring6` | Advanced/manual Spring Framework 6 adapter without the Boot starter. | `implementation("net.tikeo:tikeo-spring6:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring5` | Advanced/manual Spring Framework 5 adapter without the Boot starter. | `implementation("net.tikeo:tikeo-spring5:${TIKEO_VERSION}")` |

Gradle Kotlin DSL examples:

```kotlin
repositories {
    mavenCentral()
}

dependencies {
    // Default for new Java services: Spring Boot 4.
    implementation("net.tikeo:tikeo-spring-boot-starter:${TIKEO_VERSION}")

    // Pick exactly one of these alternatives instead when your runtime requires it:
    // implementation("net.tikeo:tikeo-spring-boot3-starter:${TIKEO_VERSION}") // Spring Boot 3
    // implementation("net.tikeo:tikeo-spring-boot2-starter:${TIKEO_VERSION}") // Spring Boot 2
    // implementation("net.tikeo:tikeo:${TIKEO_VERSION}")                      // plain Java
    // implementation("net.tikeo:tikeo-spring:${TIKEO_VERSION}")               // manual Spring Framework 7
    // implementation("net.tikeo:tikeo-spring6:${TIKEO_VERSION}")              // manual Spring Framework 6
    // implementation("net.tikeo:tikeo-spring5:${TIKEO_VERSION}")              // manual Spring Framework 5
}
```

Maven POM examples — copy **exactly one** dependency block:

```xml
<dependencies>
  <!-- Default for new Java services: Spring Boot 4 / Spring Framework 7. -->
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring-boot-starter</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>

  <!-- Spring Boot 3 / Spring Framework 6. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring-boot3-starter</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- Spring Boot 2 / Spring Framework 5. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring-boot2-starter</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- Plain Java core SDK. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- Manual Spring Framework 7 adapter without Boot auto-configuration. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- Manual Spring Framework 6 adapter without Boot auto-configuration. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring6</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- Manual Spring Framework 5 adapter without Boot auto-configuration. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring5</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->
</dependencies>
```

#### Spring Boot starter configuration

Boot starters are property-driven. They create the processor registry, Worker Tunnel client,
lifecycle hook, sandbox runner registries, and optional management client.

```yaml
tikeo:
  worker:
    enabled: true
    auto-startup: true
    dry-run: ${TIKEO_WORKER_DRY_RUN:false}
    endpoint: ${TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}
    client-instance-id: ${TIKEO_WORKER_CLIENT_INSTANCE_ID:}
    state-dir: ${TIKEO_WORKER_STATE_DIR:}
    namespace: ${TIKEO_WORKER_NAMESPACE:default}
    app: ${TIKEO_WORKER_APP:default}
    cluster: ${TIKEO_WORKER_CLUSTER:default}
    region: ${TIKEO_WORKER_REGION:default}
    capabilities: [java, spring-boot]
    labels:
      worker_pool: ${TIKEO_WORKER_POOL:java-blue}
      runtime: java

  management:
    enabled: ${TIKEO_MANAGEMENT_ENABLED:false}
    endpoint: ${TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9090}
    api-key: ${TIKEO_MANAGEMENT_API_KEY:}
    namespace: ${TIKEO_MANAGEMENT_NAMESPACE:default}
    app: ${TIKEO_MANAGEMENT_APP:default}
```

```java
import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TikeoProcessor;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

@Component
public final class BillingProcessors {
    private static final Logger log = LoggerFactory.getLogger(BillingProcessors.class);

    @TikeoProcessor("billing.reconcile")
    public TaskOutcome reconcile(TaskContext context, String payload) {
        log.info("billing reconcile started instance={} payloadBytes={}", context.instanceId(), payload.length());
        return new TaskOutcome(true, "processed:" + payload);
    }
}
```

#### Plain Java core SDK configuration

Plain Java does not use `application.yml`. Build `WorkerRegistration`, provide a `TaskProcessor`,
then start `GrpcTikeoWorkerClient` yourself.

```java
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TaskProcessor;
import net.tikeo.worker.WorkerCapabilitySet;
import net.tikeo.worker.WorkerClusterElection;
import net.tikeo.worker.WorkerRegistration;
import net.tikeo.worker.client.GrpcTikeoWorkerClient;
import java.time.Duration;
import java.util.List;
import java.util.Map;

public final class TikeoPlainJavaWorker {
    public static void main(String[] args) {
        var registration = new WorkerRegistration(
            "orders-java-1",
            "default",
            "orders",
            "local",
            "local",
            List.of("java"),
            new WorkerCapabilitySet(
                List.of("java"),
                List.of("billing.reconcile"),
                List.of(),
                List.of()
            ),
            WorkerClusterElection.enabledByDefault(),
            Map.of("worker_pool", "java-core")
        );

        TaskProcessor processor = context -> {
            // Prefer your normal SLF4J logger plus TikeoTaskLogbackAppender in Logback.
            // TaskContext.logInfo/logError remains available as a direct fallback.
            context.logInfo("plain Java task started");
            return new TaskOutcome(true, "ok:" + context.processorName());
        };

        var client = new GrpcTikeoWorkerClient(
            System.getenv().getOrDefault("TIKEO_WORKER_ENDPOINT", "http://127.0.0.1:9998"),
            registration,
            processor,
            Duration.ofSeconds(10)
        );
        Runtime.getRuntime().addShutdownHook(new Thread(client::close));
        client.start();
    }
}
```

For management API access from plain Java, create `HttpTikeoJobClient(endpoint, apiKey, namespace, app)` directly and inject the API key from your Secret store.

#### Non-Boot Spring Framework configuration

Use `tikeo-spring`, `tikeo-spring6`, or `tikeo-spring5` when you have a Spring Framework application
without Boot auto-configuration. You must define the registry and Worker client beans yourself.

```java
import net.tikeo.spring.processor.TikeoProcessorRegistry;
import net.tikeo.spring.worker.SpringTikeoTaskProcessor;
import net.tikeo.worker.WorkerClusterElection;
import net.tikeo.worker.WorkerRegistration;
import net.tikeo.worker.client.GrpcTikeoWorkerClient;
import net.tikeo.worker.client.TikeoWorkerClient;
import java.time.Duration;
import java.util.List;
import java.util.Map;
import org.springframework.context.ApplicationContext;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

@Configuration
class TikeoSpringWorkerConfiguration {
    @Bean
    TikeoProcessorRegistry tikeoProcessorRegistry() {
        return new TikeoProcessorRegistry();
    }

    @Bean(initMethod = "start", destroyMethod = "close")
    TikeoWorkerClient tikeoWorkerClient(
        ApplicationContext applicationContext,
        TikeoProcessorRegistry registry
    ) {
        registry.scanExistingBeans(applicationContext);
        var registration = new WorkerRegistration(
            "orders-spring-1",
            "default",
            "orders",
            "local",
            "local",
            List.of("java", "spring"),
            registry.workerCapabilities(),
            WorkerClusterElection.enabledByDefault(),
            Map.of("worker_pool", "spring-manual")
        );
        return new GrpcTikeoWorkerClient(
            System.getenv().getOrDefault("TIKEO_WORKER_ENDPOINT", "http://127.0.0.1:9998"),
            registration,
            new SpringTikeoTaskProcessor(registry),
            Duration.ofSeconds(10)
        );
    }
}
```

### Rust / crates.io

```bash
cargo add tikeo@${TIKEO_VERSION}
```

```toml
[dependencies]
tikeo = "${TIKEO_VERSION}"
```

### Go / Go module proxy

```bash
go get github.com/yhyzgn/tikeo/sdks/go/tikeo@v${TIKEO_VERSION}
```

```go
import "github.com/yhyzgn/tikeo/sdks/go/tikeo"
```

### Python / PyPI

```bash
python -m pip install "tikeo==${TIKEO_VERSION}"
```

```python
from tikeo import Client, local_config
```

### Node.js / npm

```bash
bun add @yhyzgn/tikeo@${TIKEO_VERSION}
npm install @yhyzgn/tikeo@${TIKEO_VERSION}
pnpm add @yhyzgn/tikeo@${TIKEO_VERSION}
```

```ts
import { Client, WorkerConfig } from "@yhyzgn/tikeo";
```

### Worker runtime configuration shared by SDKs

These are Worker registration/runtime fields common to the Java, Rust, Go, Python, and Node.js SDKs.
Language-specific wrappers may expose them as Java records, Rust structs, Go structs, Python dataclasses,
TypeScript classes, or Spring Boot properties.

| Field | Default in SDK helpers | Meaning |
| --- | --- | --- |
| `endpoint` | usually `http://127.0.0.1:9998` in demos | Worker Tunnel endpoint reachable from the worker process. |
| `clientInstanceId` / `client_instance_id` | required for core SDK helpers; Boot can generate/persist it | Stable client-side hint. The server still assigns the authoritative `worker_id`. |
| `namespace` | `default` | Tenant/environment namespace used for dispatch and management scoping. |
| `app` | `default` | Application scope used for routing and management operations. |
| `cluster` | `local` in non-Java helpers; Java Boot default is `default` | Worker cluster or environment shard. |
| `region` | `local` in non-Java helpers; Java Boot default is `default` | Worker region/zone. |
| `name` | usually the client instance id | Operator-facing worker name when the language SDK exposes it. |
| `version` | `dev` in Go/Python/Node helpers | Worker/application build version when exposed by the SDK. |
| `heartbeatEvery` / `heartbeat-interval-millis` | `10s` / `10000` | Worker lease renewal cadence. |
| `capabilities` | `[]` | Legacy/operator metadata; dispatch routing uses structured capabilities where available. |
| `structuredCapabilities` | empty | SDK processors, script runners, plugin processors, and structured tags used for routing. |
| `labels` | `{}` | Free-form operational metadata such as `worker_pool`, `runtime`, `team`, or `tier`. |
| `election.enabled` | `true` | Worker-cluster master election flag in registration. |
| `election.domain` | blank | Blank means `namespace/app/cluster/region`. |
| `election.priority` | `100` | Deterministic election priority; lower values win. |

## Run Tikeo services

Tikeo can run as Docker Compose services, direct binaries on conventional servers, systemd services,
or Kubernetes workloads. The server exposes the HTTP API/web proxy target on `9090` and the Worker
Tunnel on `9998`; the web console container exposes port `80` internally.

### Realtime console streams and proxies

Tikeo Web uses Server-Sent Events (SSE) for realtime workflow timelines, instance logs, Worker
cluster state, and dispatch queue updates. When the HTTP API is behind nginx, a load balancer, WAF,
CDN, or Kubernetes Ingress, configure the network path for long-lived `text/event-stream` responses:

- disable response buffering, proxy caching, and gzip/compression buffering for `/api/v1/**/stream`;
- set read/idle timeouts well above the 15 second SSE keep-alive cadence; `60s` is a minimum and
  `300s+` is safer for operator consoles;
- do not use SSE endpoints for health checks; use `/readyz` or `/healthz`;
- allow authenticated long-lived `GET` responses without `Content-Length`;
- redact the `token` query parameter in proxy/LB/WAF logs because browser `EventSource` cannot send
  an `Authorization` header and the Web console uses `?token=...` fallback.

See the full [SSE realtime deployment notes](docs/docs/deployment/sse-realtime.md) for nginx,
load balancer, WAF, and Kubernetes Ingress examples.

### Notification channel secret references

Notification provider credentials are configured on each Notification Center channel row, not as one
shared global provider setting. Put each row's webhook URL, signing key, routing key, SMTP URL,
SMTP password, authorization header, or app-style credential reference in that channel's
`secretRefs` object:

```json
{
  "name": "billing-feishu-prod",
  "provider": "feishu",
  "config": {"messageType": "interactive"},
  "secretRefs": {
    "url": "env:TIKEO_NOTIFICATION_CHANNEL_BILLING_FEISHU_WEBHOOK_URL",
    "signingKey": "env:TIKEO_NOTIFICATION_CHANNEL_BILLING_FEISHU_SIGNING_KEY"
  }
}
```
Use direct credentials for webhook URLs, tokens, and passwords in the drawer for convenience. Direct values are stored server-side and take effect immediately without service restarts. For deployment flexibility, you can also use `env:NAME` or bare `NAME` variables to resolve from the Server process environment.
If a plugin or app-style provider needs `appId`/`appSecret`, store those values or refs in the same channel row's `secretRefs`; the current built-in Feishu/Lark custom bot uses `url` plus optional `signingKey`.

### Docker Compose: SQLite default

Use this for the fastest local product evaluation. It builds the server and web images locally unless
you override `TIKEO_IMAGE` / `TIKEO_WEB_IMAGE`.

```bash
cp deploy/compose/tikeo.env.example .env
DOCKER_BUILDKIT=1 docker compose --env-file .env up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
open http://127.0.0.1:${TIKEO_WEB_PORT:-8080}
```

### Docker Compose: PostgreSQL

```bash
cp deploy/compose/tikeo.env.example .env
DOCKER_BUILDKIT=1 docker compose --env-file .env \
  -f docker-compose.postgres.yml \
  up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
```

### Docker Compose: MySQL

```bash
cp deploy/compose/tikeo.env.example .env
DOCKER_BUILDKIT=1 docker compose --env-file .env \
  -f docker-compose.mysql.yml \
  up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
```

### Docker without Compose

Run the control plane and web container manually when you already manage the database yourself.

```bash
docker network create tikeo || true
docker volume create tikeo-data

docker run -d --name tikeo-server --network tikeo \
  -p 9090:9090 -p 9998:9998 \
  -v tikeo-data:/data \
  -e TIKEO__STORAGE__DATABASE_URL='sqlite:///data/tikeo.db?mode=rwc' \
  yhyzgn/tikeo-server:${TIKEO_VERSION} serve --config /app/config/container.toml

docker run -d --name tikeo-web --network tikeo \
  -p 8080:80 \
  yhyzgn/tikeo-web:${TIKEO_VERSION}

curl -fsS http://127.0.0.1:9090/readyz
```

For PostgreSQL/MySQL, replace `TIKEO__STORAGE__DATABASE_URL` with the database URL exposed by your
platform and keep credentials in your secret manager.

### Non-Docker binary / VM / bare metal

Use this path for conventional servers, VMs, Supervisor, or manually managed process runners.
Production environments should prefer PostgreSQL or MySQL and durable log directories.

```bash
cargo build --release --bin tikeo
install -d ./var/lib/tikeo ./logs
cp config/dev.toml ./tikeo.toml
TIKEO__OBSERVABILITY__LOGGING__LOG_DIR=./logs \
  ./target/release/tikeo serve --config ./tikeo.toml
curl -fsS http://127.0.0.1:9090/readyz
```

Systemd deployment uses the checked-in unit files:

```bash
sudo useradd --system --home /var/lib/tikeo --shell /usr/sbin/nologin tikeo || true
sudo install -d -o tikeo -g tikeo /opt/tikeo/bin /var/lib/tikeo /var/log/tikeo /etc/tikeo
sudo install -m 0755 target/release/tikeo /opt/tikeo/bin/tikeo
sudo install -m 0644 config/container.toml /etc/tikeo/tikeo.toml
sudo install -m 0644 deploy/systemd/tikeo.env /etc/tikeo/tikeo.env
sudo install -m 0644 deploy/systemd/tikeo.service /etc/systemd/system/tikeo.service
sudo systemctl daemon-reload
sudo systemctl enable --now tikeo
systemctl status tikeo --no-pager
```

### Kubernetes manifests and operator

Use Kubernetes when the control plane should run inside a cluster and workers connect from business
namespaces or external services. Start with Helm for normal installs; use the CRD/operator path when
you want GitOps drift review through `TikeoManifest` resources.

```bash
kubectl create namespace tikeo --dry-run=client -o yaml | kubectl apply -f -
kubectl apply -f deploy/k8s/crd/tikeo-manifest-crd.yaml
kubectl get crd | grep tikeo
```

For a simple Kubernetes smoke deployment without Helm, apply the checked-in manifest:

```bash
kubectl apply -f deploy/k8s/tikeo.yaml
kubectl -n tikeo rollout status deploy/tikeo-server
kubectl -n tikeo rollout status deploy/tikeo-web
```

The operator directory contains the controller implementation, RBAC sample, and `TikeoManifest`
sample for the GitOps diff flow:

```bash
kubectl apply -f deploy/k8s/crd/tikeo-manifest-crd.yaml
kubectl -n tikeo apply -f deploy/k8s/operator/config/rbac/role.yaml
kubectl -n tikeo apply -f deploy/k8s/operator/config/samples/tikeo-manifest.yaml
```

Run the controller according to `deploy/k8s/operator/README.md` or package it as the release
operator image for your cluster.

### Helm

Install from the local chart during development:

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo \
  --create-namespace
kubectl -n tikeo rollout status deploy/tikeo-server
kubectl -n tikeo rollout status deploy/tikeo-web
```

Install a pinned release image set:

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo \
  --create-namespace \
  --set server.image.repository=yhyzgn/tikeo-server \
  --set server.image.tag=v${TIKEO_VERSION} \
  --set web.image.repository=yhyzgn/tikeo-web \
  --set web.image.tag=v${TIKEO_VERSION}
```

Production clusters should override database settings, ingress/TLS, secret references, resource
requests, log collection, and OpenTelemetry endpoints in a values file:

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo \
  --create-namespace   --values ./my-tikeo-values.yaml
```

Tikeo's production multi-pod design is the **Raft FSOD Cluster** (Fenced Slot Outbox Dispatch): a Raft-backed Server HA architecture that does not depend on external distributed locks for scheduler correctness. It combines Leader fencing, shard ownership projection, durable outbox dispatch, and Worker Tunnel gateway relay so API/Web traffic may land on any pod while task dispatch remains fenced and recoverable.

Read the dedicated guide first: [Server HA and Raft FSOD Cluster](https://docs.tikeo.net/docs/deployment/server-ha). It includes deployment diagrams, mode selection, advantages, trade-offs, configuration requirements, FSOD durability, multi-owner scheduler shard dispatch, Worker Tunnel gateway relay, and failover checks.

Raft FSOD Cluster production semantics:

| Topic | Current behavior | Operational meaning |
| --- | --- | --- |
| Server HA | Raft elects one fenced control-plane Leader, projects shard ownership across active members with health-aware minimal movement, and exposes cross-pod diagnostics probes. | More Server pods improve failover, Worker Tunnel distribution, and dispatch throughput for owned shards without remapping every shard on each membership change. |
| Dispatch durability | FSOD persists dispatch intent in `worker_dispatch_outbox` before any stream delivery. | If a gateway, relay, or Worker stream breaks, queued/delivered outbox rows can reroute or requeue instead of disappearing in pod memory. |
| Shard ownership | The runtime projects scheduler shards into `cluster_shard_ownership` with owner epoch and fencing token. | Follower shard owners can safely claim only their own job queues, workflow-node materialization, and broadcast attempts; non-owners fail closed. |
| Worker Tunnel | Workers may connect to any Server Pod; the session records `gateway_node_id`, and any shard owner uses local delivery or internal relay hints through the owning gateway. | Worker Tunnel exposure must support gRPC/HTTP2; internal peer endpoints and `cluster.transport_token` must be configured for relay. |
| Smart Gateway diagnostics | `/api/v1/cluster/diagnostics` reports `smartGateway`: local gateway node, online/local/remote Worker counts, outbox backlog, queued/reroute-pending rows, and oldest queued age. | Treat this as a safe locality/observability optimization. Correctness still comes from Raft fencing, shard ownership, durable outbox, and DB terminal-state fencing. |
| External locks | Redis/Dragonfly locks are intentionally not used for core scheduler ownership. | Optional caches can accelerate surrounding features, but scheduler correctness comes from Raft fencing, shard ownership, durable outbox, and DB terminal-state fencing. |

```bash
kubectl -n tikeo create secret generic tikeo-raft-transport \
  --from-literal=transport-token="$(openssl rand -hex 32)"
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo \
  --create-namespace \
  --values deploy/helm/tikeo/examples/values-external-postgres.yaml \
  --values deploy/helm/tikeo/examples/values-raft-ha.yaml
kubectl -n tikeo rollout status statefulset/tikeo-server

# Non-mutating rollout/rollback gate: one scheduler, active ownership, bounded skew/age.
TIKEO_SERVER_URL="https://tikeo.example.com" \
TIKEO_MANAGEMENT_API_KEY="$TIKEO_MANAGEMENT_API_KEY" \
TIKEO_EXPECTED_SERVER_REPLICAS=3 \
TIKEO_MAX_SHARD_SKEW=1 \
scripts/verify-raft-ha-rollout.sh

# Optional staging fault drill: dry-run by default, apply only with TIKEO_FAULT_MODE=apply.
scripts/raft-ha-fault-injection-drill.sh

# Local single-machine Kubernetes acceptance test with Kind:
# four Server pods, API pod != Worker gateway pod, leader deletion, before/after job dispatch.
TIKEO_KIND_E2E_KEEP=0 TIKEO_KIND_E2E_REBUILD_SERVER=1 scripts/kind-raft-ha-e2e.sh
```

The Kind run stores an auditable bundle under `.dev/reports/<run-id>/`: cluster diagnostics, FSOD metrics, DB snapshots, worker logs, instance results, Kubernetes events, and the fault-drill report. Kind is enough to validate Kubernetes StatefulSet/headless-Service/Worker-Tunnel behavior locally; still run cloud-specific checks for ingress/LB/WAF/TLS/database HA before production.

### Deployment paths

| Path | Use it when |
| --- | --- |
| `docker-compose.yml` | You want the fastest local product evaluation with SQLite. |
| `docker-compose.postgres.yml` / `docker-compose.mysql.yml` | You want a complete server + web + database stack for PostgreSQL or MySQL. |
| `deploy/systemd/` | You run Tikeo on VMs or bare-metal hosts. |
| `deploy/helm/tikeo/` | You deploy the control plane into Kubernetes. |
| `deploy/k8s/operator/` | You want CRD-based GitOps drift review. |
| `deploy/terraform/provider/` | You want manifest export/diff in Terraform workflows. |

## Observability and troubleshooting

Tikeo is designed so operators can answer the questions that matter:

- **Why did this instance dispatch or not dispatch?** Check instance logs and capability/governance messages.
- **Which worker executed the task?** Inspect instance results and broadcast worker grouping.
- **What did the script output?** Read task-scoped terminal logs, not generic process logs.
- **What changed before the failure?** Use audit logs, GitOps diff, and job/workflow versions.
- **Where is latency coming from?** Use OpenTelemetry, metrics, and SDK/server diagnostics.

## Repository map

```text
crates/            Rust server crates, scheduling, storage, worker tunnel, HTTP API
web/               React + Ant Design management console
sdks/              Java, Rust, Go, Python, and Node.js SDKs
examples/          Runnable worker demos per language
deploy/            Compose, Helm, K8s operator, Terraform provider, systemd, smoke tests
docs/              Operations, reports, localized docs, and README assets
design/            Architecture and roadmap records
scripts/           Development, seeding, release, and verification helpers
```

## Verification

```bash
cargo test --workspace
(cd web && bun run typecheck && bun run build)
(cd sdks/java && ./gradlew test --no-daemon)
(cd sdks/rust/tikeo && cargo test --all-features)
(cd sdks/go/tikeo && go test ./...)
(cd sdks/python/tikeo && python -m pytest)
(cd sdks/nodejs/tikeo && bun test && bun run build)
```

## Support Tikeo

If Tikeo saves you evaluation time or gives your team a clearer orchestration path, please consider giving the repository a ⭐. It helps more platform engineers discover the project.

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=yhyzgn/tikeo&type=Date)](https://www.star-history.com/#yhyzgn/tikeo&Date)

## License

MIT. Build boldly, operate carefully, and keep execution evidence precise.
