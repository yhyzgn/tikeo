---
title: Rust Worker SDK
description: Verified Rust SDK and Worker demo entry points.
---

# Rust Worker SDK

The Rust SDK lives under `sdks/rust/tikeo`, and the runnable worker demo lives under `examples/rust/worker-demo`.

## Verify the SDK

```bash
cargo fmt --manifest-path sdks/rust/tikeo/Cargo.toml -- --check
cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
```

## Run the demo

```bash
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

The demo is expected to connect to the Worker Tunnel endpoint from local configuration when run in live mode.
