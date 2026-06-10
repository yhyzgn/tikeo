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

## Triage order

Use a narrow triage order before changing code or configuration:

1. Confirm the process is running.
2. Confirm `healthz` and `readyz` behavior.
3. Check storage migration logs.
4. Check Worker Tunnel reachability.
5. Check worker generation/fencing token rejection messages.
6. Check Web API responses and browser console output.
7. Check audit and instance logs for governed script or policy failures.

## Common routing failures

A job may stay pending if no online worker advertises the required capability. Do not fix this by broad wildcard capabilities. Fix the processor binding, script backend, worker pool assignment, or worker runtime installation.

## Common script failures

Script governance failures are expected to be visible. Missing approval, signature mismatch, digest mismatch, denied URL/file/secret grant, timeout, and output-limit errors should be surfaced through instance logs and audit evidence.

## Escalation evidence

When reporting an issue, include the Server commit, config file path, database backend, worker SDK language/version, health/readiness output, and the relevant instance or audit id.
