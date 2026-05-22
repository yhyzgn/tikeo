# tikee

Rust Worker SDK for active outbound tikee Worker Tunnel connections.

Standalone validation from repository root:

```bash
cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features
```

This crate is self-contained for publishing: it vendors its Worker Tunnel protobuf definition under `proto/` and does not depend on server workspace crates.

Registration model: the client may provide `client_instance_id` only as a stable hint; authoritative `worker_id` is assigned by the tikee in `WorkerRegistered`.
