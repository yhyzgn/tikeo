---
title: Migrate from XXL-JOB or PowerJob
sidebar_label: Scheduler migration
description: Dry-run migration planning for XXL-JOB and PowerJob exports.
---

# Migrate from XXL-JOB or PowerJob

Tikeo includes a report-only migration planner for teams evaluating a move from XXL-JOB or PowerJob. The tool does **not** write to the Tikeo database. It reads a JSON export, maps source jobs into Tikeo `create job` drafts, and emits a reviewable report with unsupported features and manual follow-up items.

Use it before production migration to answer three questions:

1. Which source jobs can be created as Tikeo Jobs directly?
2. Which jobs need review because legacy routing, blocking, broadcast, map-reduce, or worker pinning semantics do not map one-to-one?
3. What processor names, schedules, retry policy drafts, and namespace/app targets will be used in Tikeo?

## Command

```bash
# JSON report to stdout
tikeo migrate \
  --from xxl-job \
  --input ./xxl-job-export.json \
  --namespace ops \
  --app billing

# Markdown report to a file
tikeo migrate \
  --from powerjob \
  --input ./powerjob-export.json \
  --format markdown \
  --output ./tikeo-migration-report.md
```

Accepted `--from` values:

| Value | Source |
| --- | --- |
| `xxl-job` | XXL-JOB job export records. |
| `powerjob` | PowerJob job export records. `power-job` is accepted as an alias. |

Accepted JSON shapes:

- an array of job objects;
- `{ "jobs": [...] }`;
- `{ "data": [...] }`;
- `{ "data": { "jobs": [...] } }`;
- `{ "content": [...] }`;
- one standalone job object.

## What is generated

The report contains:

| Field | Meaning |
| --- | --- |
| `source` | `xxl-job` or `powerjob`. |
| `mode` | Always `dry_run_report_only` for this MVP. |
| `summary` | Count of total, ready, needs-review, and skipped records. |
| `jobs[].tikeoJob` | Draft payload with namespace, app, name, schedule, processor, enabled flag, retry policy, and migration metadata. |
| `jobs[].unsupportedFeatures` | Source features that require human review. |
| `jobs[].warnings` | Lossy mappings or missing fields. |
| `jobs[].sourceSnapshot` | Original source fragment kept for audit/review. |

## Mapping rules

### XXL-JOB

| Source field | Tikeo draft field |
| --- | --- |
| `jobDesc` | `name` |
| `executorAppName` | `app` |
| `executorHandler` | `processorName` |
| `scheduleType=CRON` + `scheduleConf` | `scheduleType=cron`, `scheduleExpr=scheduleConf` |
| `scheduleType=FIX_RATE` | `scheduleType=fixed_rate` |
| `scheduleType=NONE` | `scheduleType=api` |
| `executorFailRetryCount` | `retryPolicy.maxAttempts = retry + 1` |
| `triggerStatus=0` | `enabled=false` |

The planner flags these for review instead of pretending they are identical: `glueType`, `executorRouteStrategy`, and `executorBlockStrategy`.

### PowerJob

| Source field | Tikeo draft field |
| --- | --- |
| `jobName` | `name` |
| `appName` | `app` |
| `processorInfo` | `processorName` |
| `timeExpressionType=2` or `CRON` | `scheduleType=cron` |
| `timeExpressionType=3` or fixed-rate names | `scheduleType=fixed_rate` |
| `timeExpressionType=4` or fixed-delay names | `scheduleType=fixed_delay` |
| `timeExpressionType=1` or `API` | `scheduleType=api` |
| `instanceRetryNum` | `retryPolicy.maxAttempts = retry + 1` |
| `status=0` | `enabled=false` |

The planner flags these for review: `executeType`, `designatedWorkers`, and `maxInstanceNum`.

## Review workflow

1. Export legacy scheduler jobs to JSON.
2. Run `tikeo migrate` and save the JSON or Markdown report.
3. Review every `needs_review` item. Translate legacy routing/blocking/pinning semantics to Tikeo Worker labels, capabilities, workflow fan-out, or concurrency policy.
4. Create a small pilot batch manually or through the Management API using the `tikeoJob` drafts.
5. Start Workers with matching `processorName` values.
6. Trigger one job at a time and compare instance logs/results with legacy behavior before switching traffic.

## Boundaries

This MVP is intentionally conservative:

- It does not connect to XXL-JOB or PowerJob databases.
- It does not create Tikeo Jobs automatically.
- It does not translate arbitrary Java executor code.
- It does not claim broadcast/map-reduce/blocking/routing semantics are equivalent.
- It keeps source snapshots in the report so humans can audit every decision.

Treat the report as a migration plan and evidence bundle, not as a one-click migration.
