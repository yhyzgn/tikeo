# scheduler-worker-sdk

Rust Worker SDK for active outbound scheduler Worker Tunnel connections.

Standalone validation from repository root:

```bash
cargo test --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml --all-features
```

This crate is self-contained for publishing: it vendors its Worker Tunnel protobuf definition under `proto/` and does not depend on server workspace crates.
