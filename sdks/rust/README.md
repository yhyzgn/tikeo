# Rust SDKs

Rust SDK packages live under `sdks/rust/<sdk-name>/` and must be independently buildable.

Current packages:

- `tikeo/`

Validation from repository root:

```bash
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
cargo build --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
```

Publishing rule: each Rust SDK crate must be self-contained for crates.io publishing and must not depend on repository-local `crates/*` path dependencies.
