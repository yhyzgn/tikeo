# Rust Worker demo 🦀

[🇨🇳 中文示例文档](../../../README.zh-CN.md#能证明产品价值的快速开始)

Runnable demo for `sdks/rust/tikeo`, aligned with Java, Go, Python, and Node.js acceptance scopes.

```bash
# Start Tikeo first from the repository root:
# ./scripts/dev.sh

cd examples/rust/worker-demo
cargo run
```

Dry-run smoke:

```bash
TIKEO_WORKER_DRY_RUN=1 cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Defaults:

- scope: `dev-alpha/orders`
- worker pool: `rust-blue`
- SDK processors: `demo.echo`, `demo.context`, `demo.bytes`, `demo.heartbeat`, `demo.fail`
- plugin processor: `type=sql`, `processorName=billing.sql-sync`
- script runners: shell, Python, JavaScript, TypeScript, PowerShell, PHP, Groovy, Rhai
- default sandbox auto path: SRT for native scripts and Deno for JavaScript/TypeScript
- Docker/Podman are explicit heavy backends and are never selected by default

Operational cautions: `TIKEO_SANDBOX_AUTO_INSTALL=0` disables sandbox tool installation; leave runtime
checks enabled in production so workers fail closed when required tools are missing.
