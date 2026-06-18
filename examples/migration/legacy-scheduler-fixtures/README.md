# Legacy scheduler migration fixtures

This directory contains tiny SQLite fixtures for local `tikeo-migrate` demos and CI compatibility tests.
They are intentionally **not** the production migration format.

Production migrations should normally run from the legacy Java/Spring worker project root and let the CLI read the real legacy scheduler database with a read-only account:

```bash
cd ./legacy-worker
tikeo-migrate plan
# or, when datasource is not in application.* / bootstrap.*:
tikeo-migrate plan \
  --from xxl-job \
  --legacy-db-url 'jdbc:mysql://legacy-db.example.com:3306/xxl_job' \
  --legacy-db-user "$LEGACY_DB_USER" \
  --legacy-db-password "$LEGACY_DB_PASSWORD"
```

## Create local fixture databases

The helper uses Python's built-in `sqlite3` module, so it does not require the `sqlite3` CLI:

```bash
./examples/migration/legacy-scheduler-fixtures/create-fixtures.sh /tmp/tikeo-migrate-fixtures
```

Then run the migration planner against each fixture:

```bash
tikeo-migrate plan \
  --from xxl-job \
  --legacy-db-url sqlite:/tmp/tikeo-migrate-fixtures/xxl-job.db \
  --output-dir /tmp/tikeo-migrate-fixtures/xxl-bundle

tikeo-migrate plan \
  --from powerjob \
  --legacy-db-url sqlite:/tmp/tikeo-migrate-fixtures/powerjob.db \
  --output-dir /tmp/tikeo-migrate-fixtures/powerjob-bundle
```

Expected evidence:

- `manifest.json` records an input origin starting with `legacy-db:sqlite://`.
- `jobs.tikeo.json` contains generated Tikeo job drafts.
- PowerJob broadcast/fan-out examples are marked `needs_review`, because their execution semantics need an explicit Tikeo workflow/routing decision before live import.

## Why SQLite exists here

`tikeo-migrate` production auto-export supports MySQL/PostgreSQL JDBC/native URLs because XXL-JOB and PowerJob are normally deployed on those engines. SQLite support is included for deterministic local demos, CI tests, and documentation examples only.
