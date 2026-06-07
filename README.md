<p align="center">
  <img src="web/src/assets/tikeo-logo.svg" alt="Tikeo logo" width="132" height="132" />
</p>

<h1 align="center">Tikeo</h1>
<p align="center"><strong>Cloud-native distributed task orchestration for jobs, workflows, worker fleets, and governed script sandboxes.</strong></p>

<p align="center">
  <a href="docs/zh-CN/README.md">🇨🇳 中文文档</a> ·
  <a href="deploy/compose/README.md">🐳 Docker Compose</a> ·
  <a href="sdks/README.md">🧩 SDKs</a> ·
  <a href="examples/README.md">🚀 Examples</a>
</p>

---

## Why Tikeo wins attention immediately ✨

Tikeo is not a trimmed-down scheduler. It is a modern orchestration platform built for teams that
need reliable task execution, rich worker governance, secure script execution, visual operations,
and SDK parity across real production languages.

| If you are evaluating... | Tikeo gives you... |
| --- | --- |
| XXL-Job style task scheduling | A Rust server core, structured worker capabilities, RBAC, API keys, audit logs, retries, workflows, topology, and script governance. |
| PowerJob style distributed workers | Worker-initiated tunnels, script sandboxes, plugin processors, worker cluster master election design, OpenTelemetry, and multi-language SDKs. |
| Kubernetes-native operation | Docker images, Compose, Helm, CRD/operator, Terraform provider, GitOps diff, OTel-ready telemetry, and database portability. |
| Embedded SDK execution | Java, Rust, Go, Python, and Node.js SDKs following the same capability, logging, sandbox, retry, and management contracts. |

## Product pillars 🧭

- ⚡ **High-performance Rust control plane** — async runtime, typed config, strict storage migrations,
  and a small operational surface.
- 🔌 **Outbound Worker Tunnel** — workers dial the server; business services do not need inbound ports
  opened just to receive tasks.
- 🧱 **Structured capabilities only** — SDK processors, plugin processors, script runners, tags, and
  worker election metadata are declared as typed fields, not fragile string conventions.
- 🛡️ **Governed script execution** — script approvals, immutable versions, digest checks, task-scoped
  logs, and sandbox auto resolution across SRT, Deno, Wasmtime/WasmEdge, V8, Docker, Podman, and custom
  backends where explicitly configured.
- 🧩 **Plugin processors and alert channels** — extend execution and notification behavior without
  hiding dispatch contracts inside worker names.
- 🗺️ **Workflow and topology visibility** — visual workflow canvas, task topology, impact analysis,
  replay-ready execution data, broadcast results, and terminal-style instance logs.
- 🔐 **Practical security model** — first-run owner bootstrap, RBAC roles, menu/action/API permission
  matrices, opaque session tokens, app-scoped API keys, tenant scopes, service accounts, and secret refs.
- 📈 **Observability by default** — INFO-level console logs, optional log-directory files, OpenTelemetry
  tracing, metrics endpoints, audit trails, and worker/task log separation.
- 🚢 **Release-ready delivery** — server/web Docker images, cross-platform binaries, SDK packaging,
  GitHub Releases, Docker Hub publishing, Maven Central, crates.io, PyPI, npm, Go modules, Helm, and
  Terraform/CRD assets.

## Tikeo vs. legacy schedulers 🥊

| Capability | Tikeo | XXL-Job | PowerJob |
| --- | --- | --- | --- |
| Worker connectivity | Outbound Worker Tunnel | Executor callback/registry patterns | Worker server model |
| Capability routing | Structured typed declarations | Mostly string/name conventions | Name/tag oriented |
| Script sandbox governance | SRT/Deno/WASM/container strategy with digest/policy checks | Limited/general script execution | Limited/general processor execution |
| Multi-language SDK parity | Java, Rust, Go, Python, Node.js | Java-centric ecosystem | Java-centric ecosystem |
| Visual workflow/topology | Built-in workflow canvas and task topology | Basic job views | Workflow support, less sandbox-oriented |
| Security model | Owner bootstrap, RBAC matrix, opaque sessions, API keys, audit | Admin/user oriented | Admin/user oriented |
| Observability | OTel, metrics, task logs, audit logs, file logs | Traditional logs | Traditional logs |
| GitOps/IaC | GitOps diff, Terraform provider, K8s CRD/operator | Not a first-class contract | Not a first-class contract |
| Deployment targets | Binary, Compose, Docker, systemd, K8s, Helm | JVM deployment | JVM deployment |

## Architecture snapshot 🏗️

```text
+---------------------+          HTTP API / WebSocket-ready UI          +----------------------+
|  React Web Console  |  <------------------------------------------->  |   Tikeo Rust Server  |
+---------------------+                                                  |  API / Scheduler     |
                                                                         |  Worker Tunnel       |
                                                                         +----------+-----------+
                                                                                    ^
                                                                                    | outbound gRPC
          +------------------+     +------------------+     +-----------------------+----------------+
          | Java Worker SDK  |     | Rust/Go Workers  |     | Python/Node Workers + Script Sandboxes |
          +------------------+     +------------------+     +----------------------------------------+
```

The server owns scheduling, persistence, governance, RBAC, workflow state, and dispatch decisions.
Workers own execution and advertise what they can safely run. Scripts are dispatched as immutable
versions and executed only by workers that explicitly expose compatible sandbox runners.

## Quick start 🚀

```bash
# Local server + web console with live logs in the terminal and .dev/*.log files
./scripts/dev.sh
```

Open <http://127.0.0.1:5173>. On a fresh database, Tikeo routes you to first-run owner setup. After
that, registration closes and users are managed from the console.

Seed development data:

```bash
./scripts/dev-seed.sh
```

Run one worker demo:

```bash
(cd examples/rust/worker-demo && cargo run)
(cd examples/go/worker-demo && go run .)
(cd examples/python/worker-demo && python -m pip install -e ../../../sdks/python/tikeo -e . && python -m tikeo_python_worker_demo)
(cd examples/nodejs/worker-demo && bun install && bun start)
(cd examples/java/spring-boot4-worker-demo && ./scripts/run-demo-worker.sh)
```

## Configuration ⚙️

Config files live in `config/` and can be overridden with `TIKEO__...` environment variables.

```toml
[observability.logging]
level = "info"
# log_dir = "./logs"

[observability.tracing]
enabled = false
# otlp_endpoint = "http://otel-collector:4318/v1/traces"
```

Supported storage URLs include SQLite, MySQL, PostgreSQL, and CockroachDB-compatible PostgreSQL wire
protocols. Prefer PostgreSQL/MySQL for shared production environments; SQLite is excellent for local
single-node demos.

## SDKs 🧩

| Language | Package | Primary use |
| --- | --- | --- |
| Java | `net.tikeo:tikeo`, Spring Boot starters | Enterprise Spring workers, management APIs, sandbox tool management. |
| Rust | `tikeo` | Native high-performance workers and script-capable runtimes. |
| Go | `github.com/yhyzgn/tikeo/sdks/go/tikeo` | Cloud-native workers, operators, and platform automation. |
| Python | `tikeo` | Data/automation workers and management integrations. |
| Node.js | `@yhyzgn/tikeo` | JavaScript/TypeScript workers and web-platform automation. |

All SDKs expose the same design language: structured capabilities, task-scoped logs, app-scoped
management clients, retry-aware job models, and INFO-level diagnostics with optional log files.

## Deployment options 🚢

- `docker-compose.yml` — SQLite default stack.
- `docker-compose.postgres.yml` / `docker-compose.mysql.yml` — database-specific stacks.
- `deploy/systemd/` — VM and bare-metal service units.
- `deploy/helm/tikeo/` — Kubernetes chart.
- `deploy/k8s/operator/` — CRD/controller for GitOps drift review.
- `deploy/terraform/provider/` — Terraform provider for manifest export and diff resources.

## Logging and observability 🔎

- Server: console logging always on, optional `observability.logging.log_dir` file output, default
  level `INFO`, and optional OTLP tracing.
- SDKs: console logging by default, optional SDK log directory/file, bridge-friendly logging APIs,
  and no stdout/stderr capture for task logs.
- Task logs: emitted through task-scoped APIs only, so broadcast execution remains precise per worker.

## Repository map 🗂️

```text
crates/            Rust server crates and storage layer
web/               React + Ant Design management console
sdks/              Java, Rust, Go, Python, and Node.js SDKs
examples/          Runnable worker demos per language
deploy/            Compose, Helm, K8s, Terraform, systemd, smoke tests
docs/              Operations, reports, and localized documentation
design/            Product architecture and roadmap records
scripts/           Development, seeding, release, and verification helpers
```

## Verification ✅

```bash
cargo test --workspace
(cd web && bun run typecheck && bun run build)
(cd sdks/java && ./gradlew test --no-daemon)
(cd sdks/rust/tikeo && cargo test --all-features)
(cd sdks/go/tikeo && go test ./...)
(cd sdks/python/tikeo && python -m pytest)
(cd sdks/nodejs/tikeo && bun test && bun run build)
```

## License 📄

Apache-2.0. Build boldly, operate carefully, and keep execution evidence precise.
