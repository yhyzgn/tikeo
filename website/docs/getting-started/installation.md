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
