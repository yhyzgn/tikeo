<p align="center">
  <img src="web/src/assets/tikeo-logo.svg" alt="Tikeo logo" width="148" height="148" />
</p>

<h1 align="center">Tikeo</h1>
<p align="center"><strong>The open-source task orchestration platform for teams that have outgrown legacy job schedulers.</strong></p>

<p align="center">
  <a href="README.zh-CN.md">🇨🇳 中文文档</a> ·
  <a href="deploy/compose/README.md">🐳 Docker Compose</a> ·
  <a href="sdks/README.md">🧩 SDKs</a> ·
  <a href="examples/README.md">🚀 Examples</a> ·
  <a href="deploy/terraform/README.md">🌍 Terraform</a> ·
  <a href="deploy/k8s/operator/README.md">☸️ Operator</a>
</p>

<p align="center">
  <a href="https://github.com/yhyzgn/tikeo/actions/workflows/ci.yml"><img alt="CI build" src="https://img.shields.io/github/actions/workflow/status/yhyzgn/tikeo/ci.yml?branch=main&style=flat-square&label=CI%20build&logo=githubactions&logoColor=white&color=2088FF" /></a>
  <a href="https://github.com/yhyzgn/tikeo/releases"><img alt="Latest release" src="https://img.shields.io/github/v/release/yhyzgn/tikeo?include_prereleases&style=flat-square&label=release&logo=github&logoColor=white&color=181717" /></a>
  <img alt="Coverage" src="https://img.shields.io/badge/coverage-report%20pending-F97316?style=flat-square&logo=codecov&logoColor=white" />
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/yhyzgn/tikeo?style=flat-square&label=license&logo=opensourceinitiative&logoColor=white&color=3DA639" /></a>
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
| **5 production SDK tracks** | **Java · Rust · Go · Python · Node.js** workers follow one contract instead of one Java-first executor model. |
| **Outbound Worker Tunnel** | Workers connect out; production services do **not** need inbound task-execution ports. |
| **Structured capability routing** | Dispatch matches typed **SDK processors**, **plugin processors**, and **script runners**. No magic string parsing. |
| **Sandbox-first script jobs** | `auto` selects **SRT** for native scripts and **Deno** for JS/TS, with **WASM/V8/container** paths available explicitly. |
| **Workflow + topology UX** | Visual workflow canvas, dependency topology, impact analysis, replay data, and per-worker broadcast results. |
| **Operations-grade evidence** | **Retries**, **misfire policy**, **task logs**, **audit logs**, **OpenTelemetry**, metrics, and file logs answer “what happened?” |
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

This is not a “feature-count flex.” It is the difference between a job scheduler and an orchestration
platform.

| Evaluation axis | Tikeo | XXL-Job | PowerJob |
| --- | --- | --- | --- |
| **Platform role** | ✅ **Full orchestration platform**: jobs, workflows, workers, scripts, plugins, RBAC, observability, IaC. | Mature Java job scheduler. | Mature Java distributed job platform. |
| **Worker connection model** | ✅ **Outbound Worker Tunnel** with lease/fencing and structured registration. | Executor registration/callback style. | Worker server model. |
| **Routing contract** | ✅ **Typed SDK/plugin/script capabilities**; no convention parsing. | Name/string oriented. | Name/tag oriented. |
| **Language ecosystem** | ✅ **Java · Rust · Go · Python · Node.js** SDK parity. | Primarily Java ecosystem. | Primarily Java ecosystem. |
| **Script execution** | ✅ **Governed versions + digest checks + SRT/Deno/WASM/V8/container** strategy. | Script execution exists but is not a full sandbox governance product. | Processor-focused; sandbox governance is not the center. |
| **Workflow UX** | ✅ **Workflow canvas + topology + impact analysis + replay-ready execution data.** | Basic scheduling-centric views. | Workflow support, less focused on typed sandbox + SDK parity. |
| **Security model** | ✅ **Owner bootstrap, RBAC matrix, opaque sessions, API keys, tenant scopes, audit trails.** | Admin/user model. | Admin/user model. |
| **Observability** | ✅ **OpenTelemetry, metrics, task logs, file logs, audit logs, worker grouping.** | Traditional operations/logs. | Traditional operations/logs. |
| **Cloud-native assets** | ✅ **Docker, Compose, Helm, K8s CRD/operator, Terraform provider, GitOps diff.** | Deployable, but not GitOps/IaC-first. | Deployable, but not GitOps/IaC-first. |
| **Best fit** | Teams building an internal orchestration platform, not just a cron replacement. | Java teams wanting a familiar scheduler. | Java teams wanting distributed job execution. |

**Short version:** choose Tikeo when you want a modern orchestration control plane; choose legacy
schedulers only when you intentionally want a narrower Java-first scheduler.

### Evaluation checklist

If your scheduler shortlist includes these requirements, Tikeo should move to the top:

- [x] **Multi-language workers** without losing one platform contract.
- [x] **Workflow + topology visualization** instead of job-list-only operations.
- [x] **Script sandbox governance** with explicit backend selection and default lightweight auto mode.
- [x] **RBAC + API-Key + audit** for real admin operations.
- [x] **OpenTelemetry + metrics + durable logs** for production troubleshooting.
- [x] **Helm + Terraform + K8s CRD/operator** for platform engineering teams.

## Architecture

<p align="center">
  <img src="docs/assets/tikeo-architecture.svg" alt="Tikeo architecture diagram" width="100%" />
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

## Quick start that proves the product

### 1. Start the control plane

```bash
./scripts/dev.sh
```

This starts the Rust server and React web console, streams logs to the terminal, and also writes local
logs under `.dev/`.

Open <http://127.0.0.1:5173>. A fresh database routes you to first-run owner setup. After the owner is
created, registration closes and users/roles are managed inside the console.

### 2. Seed real evaluation data

```bash
./scripts/dev-seed.sh
```

The seed data gives you namespaces, apps, sample jobs, scripts, workflows, audit records, and instance
logs so you can evaluate the console immediately instead of staring at an empty product.

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

```toml
[storage]
database_url = "postgres://tikeo:tikeo@postgres:5432/tikeo"

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

| Language | Package | Best for | Logging contract |
| --- | --- | --- | --- |
| Java | `net.tikeo:tikeo`, Spring Boot starters | Enterprise Spring workers and management automation. | SLF4J diagnostics; task logs through `TaskContext`. |
| Rust | `tikeo` | Native workers, high-performance runtimes, sandbox-capable services. | `SdkLogConfig`, console + optional `tikeo-sdk.log`. |
| Go | Go module | Platform services, operators, cloud-native workers. | `Logger` bridge, console + optional `tikeo-sdk.log`. |
| Python | `tikeo` | Data jobs, automation, scripting-friendly workers. | stdlib `logging`, console + optional `tikeo-sdk.log`. |
| Node.js | `@yhyzgn/tikeo` | JS/TS workers and web-platform automation. | `configureSdkLogging`, console + optional `tikeo-sdk.log`. |

All SDKs follow the same rule: SDK diagnostics describe worker/runtime lifecycle; task logs describe a
specific job instance. That separation prevents unrelated process noise from polluting execution logs.

## Install SDKs from central registries

Use one package per worker service. Every SDK follows the same platform contract: outbound Worker
Tunnel, structured capabilities, task-scoped logs, retry/result reporting, management APIs, and
sandbox auto behavior.

| Language | Central registry | Package name | Current install target |
| --- | --- | --- | --- |
| Java | Maven Central | `net.tikeo:*` | `0.1.0` release artifacts; local development may use `0.1.0-SNAPSHOT`. |
| Rust | crates.io | `tikeo` | `0.1.0` |
| Go | Go module proxy | `github.com/yhyzgn/tikeo/sdks/go/tikeo` | tag-based, for example `v0.1.0` |
| Python | PyPI | `tikeo` | `0.1.0` |
| Node.js | npm | `@yhyzgn/tikeo` | `0.1.0` |

### Java / Maven Central

Choose exactly one runtime adapter for each application. Plain Java workers only need the core SDK;
Spring applications should use the starter matching their Spring Boot generation.

| Artifact | Use it for |
| --- | --- |
| `net.tikeo:tikeo` | Plain Java workers, management clients, sandbox tooling, and low-level Worker Tunnel usage. |
| `net.tikeo:tikeo-spring` | Spring Framework 7 adapter used by Spring Boot 4 applications. |
| `net.tikeo:tikeo-spring6` | Spring Framework 6 adapter used by Spring Boot 3 applications. |
| `net.tikeo:tikeo-spring5` | Spring Framework 5 adapter used by Spring Boot 2 applications. |
| `net.tikeo:tikeo-spring-boot-starter` | Spring Boot 4 auto-configuration starter. |
| `net.tikeo:tikeo-spring-boot3-starter` | Spring Boot 3 auto-configuration starter. |
| `net.tikeo:tikeo-spring-boot2-starter` | Spring Boot 2 auto-configuration starter. |

Gradle Kotlin DSL:

```kotlin
repositories {
    mavenCentral()
}

dependencies {
    // Plain Java worker / management client.
    implementation("net.tikeo:tikeo:0.1.0")

    // Pick ONE starter when using Spring Boot.
    implementation("net.tikeo:tikeo-spring-boot-starter:0.1.0")  // Spring Boot 4
    // implementation("net.tikeo:tikeo-spring-boot3-starter:0.1.0") // Spring Boot 3
    // implementation("net.tikeo:tikeo-spring-boot2-starter:0.1.0") // Spring Boot 2
}
```

Maven:

```xml
<dependencies>
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo</artifactId>
    <version>0.1.0</version>
  </dependency>
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring-boot-starter</artifactId>
    <version>0.1.0</version>
  </dependency>
</dependencies>
```

Spring Boot worker configuration:

```yaml
tikeo:
  worker:
    enabled: true
    auto-startup: true
    endpoint: http://127.0.0.1:9998
    namespace: dev-alpha
    app: orders
    worker-pool: java-green
```

### Rust / crates.io

```bash
cargo add tikeo@0.1.0
```

```toml
[dependencies]
tikeo = "0.1.0"
```

### Go / Go module proxy

```bash
go get github.com/yhyzgn/tikeo/sdks/go/tikeo@v0.1.0
```

```go
import "github.com/yhyzgn/tikeo/sdks/go/tikeo"
```

### Python / PyPI

```bash
python -m pip install "tikeo==0.1.0"
```

```python
from tikeo import Client, local_config
```

### Node.js / npm

Bun is the default package runner in this repository:

```bash
bun add @yhyzgn/tikeo@0.1.0
```

npm and pnpm users can install the same package from the public npm registry:

```bash
npm install @yhyzgn/tikeo@0.1.0
pnpm add @yhyzgn/tikeo@0.1.0
```

```ts
import { Client, WorkerConfig } from "@yhyzgn/tikeo";
```

## Run Tikeo services

Tikeo can run as Docker Compose services, direct binaries on conventional servers, systemd services,
or Kubernetes workloads. The server exposes the HTTP API/web proxy target on `9090` and the Worker
Tunnel on `9998`; the web console container exposes port `80` internally.

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
  -f docker-compose.yml \
  -f docker-compose.postgres.yml \
  up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
```

### Docker Compose: MySQL

```bash
cp deploy/compose/tikeo.env.example .env
DOCKER_BUILDKIT=1 docker compose --env-file .env \
  -f docker-compose.yml \
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
  yhyzgn/tikeo-server:0.1.0 serve --config /app/config/container.toml

docker run -d --name tikeo-web --network tikeo \
  -p 8080:80 \
  yhyzgn/tikeo-web:0.1.0

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
  --create-namespace   --set server.image.repository=yhyzgn/tikeo-server   --set server.image.tag=0.1.0   --set web.image.repository=yhyzgn/tikeo-web   --set web.image.tag=0.1.0
```

Production clusters should override database settings, ingress/TLS, secret references, resource
requests, log collection, and OpenTelemetry endpoints in a values file:

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo \
  --create-namespace   --values ./my-tikeo-values.yaml
```

### Deployment paths

| Path | Use it when |
| --- | --- |
| `docker-compose.yml` | You want the fastest local product evaluation with SQLite. |
| `docker-compose.postgres.yml` / `docker-compose.mysql.yml` | You want to validate real database portability. |
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

## License

MIT. Build boldly, operate carefully, and keep execution evidence precise.
