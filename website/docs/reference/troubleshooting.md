---
title: Troubleshooting
description: First checks for local Tikeo evaluation failures.
---

# Troubleshooting

## Server does not start

Run:

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

Check for configuration parse errors, database connection errors, or occupied ports.

## Health check fails

```bash
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/readyz
```

If `healthz` fails, the Server is not reachable. If `readyz` fails, inspect storage, migration, or dependency readiness logs.

## Worker is invisible

- Confirm the Worker Tunnel endpoint is reachable from the Worker process.
- Confirm the worker advertises real capabilities.
- Confirm generation/fencing token checks are not rejecting stale heartbeats or results.
- Inspect worker session history in the Web console.

## Docker image build is slow

Server image validation compiles the Rust workspace and can take significantly longer than Web image validation on a cold GitHub runner.
