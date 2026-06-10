---
title: Installation
description: Toolchains, repository surfaces, version baselines, first-time bootstrap prerequisites, and verification commands for evaluating Tikeo.
---

# Installation

This page prepares a workstation or CI runner for a real Tikeo evaluation. It does not stop at cloning the repository; it explains which toolchain belongs to which project surface, which commands prove the surface is usable, where first-time Owner bootstrap happens, and which failures usually mean the environment is incomplete.

## Toolchain matrix

| Surface | Directory | Required tools | Why it is needed |
| --- | --- | --- | --- |
| Server and Rust crates | repository root, `crates/*`, `src/main.rs` | Rust 1.95+ with Cargo | Builds the `tikeo` binary, storage migrations, Worker Tunnel server, HTTP API, and core tests. |
| Web console | `web/` | Bun, Node-compatible runtime | Builds and tests the React/TypeScript/Ant Design operator console. Use Bun for repository commands. |
| Docs site | `docs/` | Bun | Builds the Docusaurus documentation site and docs Docker image. |
| Java SDK and demos | `sdks/java`, `examples/java/*` | Java runtime 17+, JDK toolchain configured by Gradle; repository builds use Java toolchain properties | Builds core Java SDK, Spring modules, Boot 2/3/4 starters, and demo Workers. |
| Go SDK and demo | `sdks/go/tikeo`, `examples/go/worker-demo` | Go 1.26+ baseline from repository docs and CI policy | Builds Go Worker SDK, Management client, and demo Worker. |
| Python SDK and demo | `sdks/python/tikeo`, `examples/python/worker-demo` | Python 3.11+ | Builds the Python package, Worker client, script helpers, and tests. |
| Node.js SDK and demo | `sdks/nodejs/tikeo`, `examples/nodejs/worker-demo` | Bun for repo commands; Node.js 24+ package baseline | Builds TypeScript SDK, demo Worker, Management client, and generated dist files. |
| Containers | root `Dockerfile`, `web/Dockerfile`, `docs/Dockerfile` | Docker with BuildKit | Builds Server, Web, and Docs images. |
| Kubernetes | `deploy/helm/tikeo`, `deploy/k8s` | `kubectl`, `helm` for live clusters | Installs Server/Web only; business Workers connect outbound. |

## Version baselines

The README badges summarize public package baselines, but the source of truth for local work is the repository itself:

- Root Cargo workspace uses Rust 2024 edition and the checked-in `Cargo.lock`.
- `docs/package.json` and `web/package.json` are both Bun-driven project modules; use `bun` and `bunx`, not npm/yarn, unless a release script explicitly says otherwise.
- Java modules are declared in `sdks/java/settings.gradle.kts`: `tikeo`, `tikeo-spring`, `tikeo-spring5`, `tikeo-spring6`, `tikeo-spring-boot-starter`, `tikeo-spring-boot2-starter`, and `tikeo-spring-boot3-starter`.
- Python package metadata lives in `sdks/python/tikeo/pyproject.toml` and requires Python `>=3.11`.
- Node package metadata lives in `sdks/nodejs/tikeo/package.json`; repository scripts use Bun and the package declares a Node.js runtime baseline for consumers.
- Config defaults are not guessed from examples; they are loaded from `crates/tikeo-config/src/lib.rs` and can be overridden with `TIKEO__...` environment variables.

## Repository surfaces: clone and inspect

```bash
git clone https://github.com/yhyzgn/tikeo.git
cd tikeo
find . -maxdepth 2 -type d | sort | sed -n '1,80p'
```

The important top-level directories are:

| Path | Purpose |
| --- | --- |
| `config/` | Server TOML examples for dev/container/PostgreSQL/MySQL/raft shape. |
| `crates/` | Rust library crates. Keep modules split by responsibility. |
| `src/main.rs` | Server binary entrypoint using `tikeo_server::run_cli()`. |
| `web/` | Operator console module. |
| `docs/` | Documentation site module and docs Docker image. |
| `sdks/` | Language SDKs with independent package metadata. |
| `examples/` | Runnable Worker demos. |
| `deploy/` | Compose, Helm, Kubernetes, systemd, Terraform, and smoke assets. |
| `scripts/` | Local dev, seed, verification, and management trigger smoke scripts. |
| `.github/tests/` | Contract tests that keep workflows/docs/deploy surfaces honest. |

## Verify core tools

Run these before blaming Tikeo for a failed local start:

```bash
rustc --version
cargo --version
bun --version
docker --version || true
go version || true
java -version || true
python3 --version || python --version
```

If you only plan to evaluate Server + one Node.js Worker, you do not need every language tool installed. If you plan to run the full cross-language smoke suite, install every language tool first.

## Install dependencies by module

Rust dependencies are resolved by Cargo from the root lockfile:

```bash
cargo fetch
cargo test --workspace --all-features --no-run
```

Web and docs dependencies must use Bun:

```bash
cd web
bun install --frozen-lockfile
cd ../docs
bun install --frozen-lockfile
cd ..
```

SDK modules are independent package surfaces. Examples:

```bash
(cd sdks/nodejs/tikeo && bun install --frozen-lockfile)
(cd examples/nodejs/worker-demo && bun install --frozen-lockfile)
(cd sdks/python/tikeo && python3 -m pip install -e '.[test]')
(cd sdks/go/tikeo && go test ./... -count=1)
./sdks/java/gradlew -p sdks/java test --no-daemon
```

## First-time bootstrap

Starting the Server only proves listeners and migrations. To operate the Web/API as an authenticated human, bootstrap the first Owner exactly once. The relevant HTTP endpoints are source-backed by `crates/tikeo-server/src/http/auth.rs` and routed under `/api/v1/auth/bootstrap`.

Check whether bootstrap is open:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/auth/bootstrap | jq .
```

Register the first Owner in a local disposable environment:

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/bootstrap/register \
  -H 'content-type: application/json' \
  -d '{"username":"bootstrap_admin","email":"bootstrap.admin@example.com","password":"Tikeo@2026!","confirmPassword":"Tikeo@2026!"}' | jq .
```

Login later with:

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"bootstrap_admin","password":"Tikeo@2026!"}' | jq .data.token
```

Do not reuse these sample credentials outside an isolated local DB. For CI smoke scripts, the script creates an isolated DB under `.dev/reports/...` and is responsible for its own temporary credentials.

## Verification commands

A serious local baseline should run at least:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
python3 scripts/check-source-size.py
python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24
```

For Web:

```bash
cd web
bun run typecheck
bun run test
bun run build
cd ..
```

For Docs:

```bash
cd docs
bun run docs:typecheck
bun run docs:build
cd ..
docker build -f docs/Dockerfile docs -t tikeo-docs:local
```

For source-backed docs contracts:

```bash
python3 .github/tests/docs_site_contract_test.py
python3 .github/tests/workflow_contract_test.py
python3 .github/tests/management_smoke_contract_test.py
```

## Minimal local start

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

From another shell:

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

`config/dev.toml` binds HTTP to `0.0.0.0:9090`, Worker Tunnel to `0.0.0.0:9998`, and SQLite to `tikeo-dev.db` with timestamp offset `+08:00`. The library default is `+00:00`; read [Configuration reference](../reference/configuration) before comparing timestamps across environments.

## Common installation failures

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| `bun install --frozen-lockfile` fails in docs or web | Wrong package manager, stale registry, or network auth problem | Use Bun; ensure lockfiles resolve to public registry URLs unless intentionally configured otherwise. |
| Worker demo cannot connect | Server not listening on Worker Tunnel port `9998`, wrong endpoint, or TLS mismatch | Start Server first; use `TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998` for local plaintext. |
| API returns unauthorized | Owner not bootstrapped or token missing | Bootstrap first Owner, login, pass bearer token for human management routes. |
| SDK Management client returns unauthorized | App-scoped SDK API key missing or invalid | Create service account + API key; pass `x-tikeo-api-key`, usually from `TIKEO_MANAGEMENT_API_KEY`. |
| SQLite lock/timeouts | Multiple test/server processes using same DB | Use isolated DB paths for smokes; clean stale local processes. |
| Helm examples fail locally | `helm` or CRDs/controllers missing | Use `helm template`/contract tests locally; run live controller smokes only in a cluster with the controller installed. |

## Next step

Continue to [Quickstart](./quickstart) for a step-by-step Server + Web + Worker + SDK Management API run. If you already know the runtime and only need a config key, jump to [Configuration reference](../reference/configuration).
