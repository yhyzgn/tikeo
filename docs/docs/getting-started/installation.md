---
title: Installation
description: Toolchain and local checkout requirements for evaluating Tikeo.
---

# Installation

Use this page to prepare a local evaluation environment. The docs site is scaffolded for public docs; the commands below point to verified repository entry points.

## Required tools

| Surface | Runtime |
|---|---|
| Server | Rust 1.95+ |
| Web console | Bun + modern Node-compatible environment |
| Java SDK/demo | Java 17+ runtime, Java 21 toolchain for repository builds |
| Go SDK/demo | Go 1.26+ |
| Python SDK/demo | Python 3.11+ |
| Node.js SDK/demo | Bun / Node.js 24+ CI surface |

## Clone

```bash
git clone https://github.com/yhyzgn/tikeo.git
cd tikeo
```

## Verify core tools

```bash
cargo --version
bun --version
go version
java -version
python --version
```

## Recommended first check

```bash
cargo test --workspace --all-features
bun run --cwd web test
```

For a faster path, continue to [Quickstart](./quickstart).

## Evaluation depth notes

A reliable Tikeo evaluation should verify each runtime surface separately before combining them. Start with Rust because the Server, storage migrations, scheduler, Worker Tunnel, and most integration tests live in the Rust workspace. Then verify Web because the console is the primary operator experience. Finally verify at least one Worker SDK so the evaluation covers the outbound-only Worker Tunnel rather than only the Server API.

## Recommended local baseline

Run these checks before demoing the system to another engineer:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-features
bun run --cwd web typecheck
bun run --cwd web test
```

For docs-site work, verify the separate docs app:

```bash
cd docs
bun install --frozen-lockfile
bun run docs:typecheck
bun run docs:build
```

## Common setup mistakes

- Running a Worker before the Server tunnel listener is available on `9998`.
- Using a stale database shape instead of running the normal migration path.
- Assuming Web development assets are embedded in the Server binary during local frontend development.
- Treating Python or Node.js examples as placeholders; they are present in CI and should be documented only from verified commands.

## Next decision

Choose the evaluation path: local binary for fastest feedback, Docker Compose for runtime packaging, or Helm/Kubernetes for production deployment planning.
