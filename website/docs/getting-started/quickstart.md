---
title: "Quickstart: Server + Web + Worker"
description: Start Tikeo locally and connect a verified Worker demo.
---

# Quickstart: Server + Web + Worker

This quickstart focuses on a single-machine evaluation. It uses the same repository commands that are maintained in project memory and CI.

## 1. Start the Server

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

Expected checks:

```bash
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/readyz
```

The HTTP API and embedded Server endpoints listen on `9090`. The Worker Tunnel listener uses `9998` in the default development shape.

## 2. Open the Web console

Run the Web app from the repository when developing UI behavior:

```bash
cd web
bun install --frozen-lockfile
bun run dev
```

For production-style packaging, use `bun run build` in `web/` and deploy the generated static assets with the chosen hosting/runtime path.

## 3. Connect a Worker demo

Rust Worker demo:

```bash
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Go Worker demo:

```bash
cd examples/go/worker-demo
go test ./... -count=1
```

Java Spring Boot demos are split by Spring Boot compatibility line:

```bash
cd examples/java/spring-boot3-worker-demo
./gradlew test --no-daemon
```

## 4. Inspect evidence

Use the Web console or HTTP API to inspect workers, jobs, instances, logs, and audit evidence. Worker visibility must not depend only on in-memory registration; current Server behavior persists worker session snapshots.

## Next

- [Seed demo data](./seed-demo-data)
- [Worker Tunnel concept](../concepts/worker-tunnel)
