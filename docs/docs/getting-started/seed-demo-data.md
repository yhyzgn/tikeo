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

## Better demo-data strategy

Use demo data to teach actual product strengths: scope isolation, worker capability matching, workflow replay, script governance, alert delivery, and auditability. A good demo should show a successful path and one intentional governed failure so evaluators can see how Tikeo behaves under real operational pressure.

## Suggested demo narrative

1. Create a namespace/app pair for the demo.
2. Connect one Rust or Go worker with a clear processor capability.
3. Create an API-triggered job and run it.
4. Inspect instance attempts and logs.
5. Create a small workflow that references the job.
6. Show worker session history and audit evidence.
7. Trigger a policy-denied script release or missing capability case and show the visible failure reason.

## Data quality rule

Do not seed random rows just to make dashboards look full. Use typed APIs or committed demo scripts so the resulting data has real relationships and audit evidence.
