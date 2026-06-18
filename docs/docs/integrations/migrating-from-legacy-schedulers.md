---
title: Migrate from XXL-JOB or PowerJob
sidebar_label: Scheduler migration
description: Dry-run migration planning for XXL-JOB and PowerJob exports.
---

# Migrate from XXL-JOB or PowerJob

Tikeo provides a dedicated `tikeo-migrate` CLI for teams moving from XXL-JOB or PowerJob. The default command, `plan`, is non-destructive: it reads a JSON export, maps source jobs into Tikeo `create job` drafts, optionally scans a Java/Spring worker project, and writes a migration bundle with reports, Java dependency guidance, handler annotation patches, unsupported features, and manual follow-up items.

Use it before production migration to answer three questions:

1. Which source jobs can be created as Tikeo Jobs directly?
2. Which jobs need review because legacy routing, blocking, broadcast, map-reduce, or worker pinning semantics do not map one-to-one?
3. What processor names, schedules, retry policy drafts, and namespace/app targets will be used in Tikeo?

## Command

### Recommended convention-first flow

Put the legacy export JSON in the legacy worker project root and run the tool from that directory. In this layout the migration planner needs no manual discovery parameters:

```bash
cd ./legacy-worker

# Build a complete non-destructive migration bundle in ./.tikeo-migration
tikeo-migrate plan

# Review the generated bundle, then dry-run API application.
# apply-data also defaults --bundle to ./.tikeo-migration.
tikeo-migrate apply-data \
  --endpoint http://127.0.0.1:9090 \
  --api-key "$TIKEO_MIGRATION_API_KEY" \
  --dry-run
```

Auto-detection rules:

| Input | Convention |
| --- | --- |
| Project root | The current directory when it contains `pom.xml`, `build.gradle`, or `build.gradle.kts`. |
| Export file | One clear JSON file named like `xxl-job-export.json`, `xxljob-export.json`, `powerjob-export.json`, `power-job-export.json`, `jobs-export.json`, or a matching JSON file under `export/`, `exports/`, or `migration/`. |
| Source scheduler | File name first, then JSON content such as XXL-JOB `executorHandler`/`jobDesc`/`scheduleConf` or PowerJob `processorInfo`/`timeExpressionType`/`instanceRetryNum`. |
| Bundle output | `./.tikeo-migration`. |

If more than one possible export file is found, or the source cannot be inferred safely, the command fails with an explicit message instead of guessing.

### Override flags for non-standard layouts

```bash
tikeo-migrate plan \
  --from xxl-job \
  --input ./exports/jobs.json \
  --project ./legacy-worker \
  --output-dir ./migration-bundle \
  --namespace ops \
  --app billing

tikeo-migrate apply-data \
  --bundle ./migration-bundle \
  --endpoint http://127.0.0.1:9090 \
  --api-key "$TIKEO_MIGRATION_API_KEY" \
  --dry-run
```

`--from` accepts:

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

The bundle contains:

| Field | Meaning |
| --- | --- |
| `manifest.json` | Complete bundle manifest with data, code, and checklist sections. |
| `jobs.tikeo.json` / `jobs.tikeo.md` | Job migration report with total, ready, needs-review, and skipped records. |
| `data-import-plan.json` | Ready and needs-review Tikeo job drafts split for controlled application. |
| `java-project-plan.json` / `.md` | Detected build system, Spring Boot major version, recommended Tikeo artifact, handler candidates, and review notes. |
| `java-patches/*.patch` | Review-first dependency and handler annotation patch guidance. |
| `CHECKLIST.md` | Human acceptance flow for branch review, staging import, one-job trigger, and dual-run comparison. |

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

1. Export legacy scheduler jobs to JSON and place the file in the legacy worker project root when possible.
2. From that project root, run `tikeo-migrate plan`. Use `--input`, `--from`, `--project`, or `--output-dir` only as overrides for non-standard layouts.
3. Review every `needs_review` item. Translate legacy routing/blocking/pinning semantics to Tikeo Worker labels, capabilities, workflow fan-out, or concurrency policy.
4. Apply generated Java patches on a branch, add the recommended starter dependency, and adapt complex handler signatures manually.
5. Run `tikeo-migrate apply-data --dry-run`, then apply ready jobs to staging without `--dry-run`.
6. Start Workers with matching `processorName` values.
7. Trigger one job at a time and compare instance logs/results with legacy behavior before switching traffic.

## Boundaries

This MVP is intentionally conservative:

- `plan` does not connect to XXL-JOB or PowerJob databases.
- `plan` does not create Tikeo Jobs or edit legacy source files.
- `apply-data` is the only command that can call the Tikeo Management API, and it supports `--dry-run`.
- Generated Java patches cover dependency insertion and handler annotation guidance; arbitrary executor/business code still requires review.
- It does not claim broadcast/map-reduce/blocking/routing semantics are equivalent.
- It keeps source snapshots in the report so humans can audit every decision.

Treat the bundle as a controlled migration plan and evidence package, not as blind one-click migration.
