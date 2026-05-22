# Rust Worker Demo

Runnable demo for `sdks/rust/tikee`.

Build and run independently from the repository root:

```bash
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Environment variables:

- `TIKEE_WORKER_ENDPOINT` defaults to `http://0.0.0.0:9998`
- `TIKEE_WORKER_INSTANCE_ID` defaults to `rust-demo-worker`

The demo is a dry-run configuration smoke test by default. Replace it with `WorkerClient::connect()` when testing against a live tikee Worker Tunnel.
