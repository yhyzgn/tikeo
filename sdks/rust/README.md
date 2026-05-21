# Rust SDKs

Rust SDK packages live under `sdks/rust/<sdk-name>/` and must be independently buildable.

Current packages:

- `scheduler-worker-sdk/`

Validation from repository root:

```bash
cargo test --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml --all-features
cargo build --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml --all-features
```
