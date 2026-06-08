---
title: Seed demo data
description: Safe demo-data paths for evaluating Tikeo without fake production claims.
---

# Seed demo data

The public docs should only advertise demo paths that are backed by real commands, tests, or recorded browser evidence.

## Current safe demo paths

- Use the local Server configuration in `config/dev.toml`.
- Use verified Worker demos under `examples/rust`, `examples/go`, and `examples/java`.
- Use the promotional browser walkthrough artifact only as visual marketing evidence, not as an automated acceptance test.

## Create a simple job through HTTP

After authenticating in a local development session, create jobs through the typed API. The exact payload depends on the currently enabled auth and scope setup.

```bash
curl -fsS http://0.0.0.0:9090/api/v1/jobs   -H 'content-type: application/json'   -d '{"namespace":"default","app":"demo","name":"manual-demo"}'
```

If authorization is enabled, include the session or API-key header required by your local configuration.

## What not to seed

Do not insert database rows by hand for public docs. Prefer typed APIs or committed demo scripts so audit, RBAC, and migration boundaries stay visible.
