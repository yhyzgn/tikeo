# Rust Worker Demo

Runnable demo for `sdks/rust/scheduler-worker-sdk`.

Build and run independently from the repository root:

```bash
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Environment variables:

- `SCHEDULER_WORKER_ENDPOINT` defaults to `http://0.0.0.0:9998`
- `SCHEDULER_WORKER_ID` defaults to `rust-demo-worker`

The demo is a dry-run configuration smoke test by default. Replace it with `WorkerClient::connect()` when testing against a live scheduler Worker Tunnel.
