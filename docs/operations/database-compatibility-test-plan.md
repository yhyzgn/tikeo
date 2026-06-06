# Database compatibility test plan

## Goal

Tikeo storage must run the same schema migration and repository behavior on:

- SQLite for local/dev/single-node deployments.
- PostgreSQL 13+; validation asset currently runs PostgreSQL 16.
- MySQL 8.0+ including the current MySQL 8.4 LTS line; validation asset currently runs MySQL 8.4 with `utf8mb4`.

## Compatibility contract

| Area | Required behavior | Test asset |
| --- | --- | --- |
| Backend features | `tikeo-storage` builds with SeaORM SQLite/PostgreSQL/MySQL sqlx backends enabled | `cargo test -p tikeo-storage --test database_compat` |
| Empty schema bootstrap | `connect_and_migrate` creates the complete schema and RBAC seed data on an empty DB | `scripts/db-compat-smoke.sh` |
| Migration idempotency | Re-running `connect_and_migrate` against the same DB does not fail on existing tables/indexes/default RBAC rows | `database_compat.rs` reruns migration per backend |
| Scope metadata | namespace/app/worker_pool CRUD works and uniqueness indexes accept normal names | `ScopeRepository` smoke |
| Job definitions | API job create/list/version snapshot persists schedule windows, calendar JSON, processor binding, booleans, integers | `JobRepository` smoke |
| Plugin JSON fields | processor/alert-channel nested JSON survives round trip including arrays and optional fields | `PluginRepository` smoke |
| Script governance fields | script content, language, limits, policy JSON, env allow-list and boolean flags survive round trip | `ScriptRepository` smoke |
| Instance queue path | pending instance creation, dispatch claim and terminal status update work transactionally | `JobInstanceRepository` smoke |
| Logs and Unicode | instance log append/list preserves worker id, sequence, unicode text and ordering | `JobInstanceLogRepository` smoke |
| Timestamp convention | timestamps remain RFC3339 strings; configured display offset is server-side behavior, not DB-specific session time zone | repository smoke with `+08:00` schedule fields |
| SQLite-specific repair | SQLite may run local schema compatibility/backfill helpers; PostgreSQL/MySQL must rely on migrations only | `connect_and_migrate` backend gating |

## Executable assets

### Local SQLite only

```bash
cargo test -p tikeo-storage --test database_compat sqlite_database_compatibility_smoke -- --nocapture
```

### Full local matrix with Docker

```bash
./scripts/db-compat-smoke.sh
```

The script starts `deploy/compose/database-compat-compose.yml` when Docker is available, then runs:

- `sqlite::memory:` smoke.
- `postgres://tikeo:tikeo@127.0.0.1:15432/tikeo` smoke.
- `mysql://tikeo:tikeo@127.0.0.1:13306/tikeo` smoke.

### External database endpoints

```bash
export TIKEO_DB_COMPAT_COMPOSE=false
export TIKEO_TEST_POSTGRES_URL='postgres://user:pass@host:5432/tikeo'
export TIKEO_TEST_MYSQL_URL='mysql://user:pass@host:3306/tikeo'
./scripts/db-compat-smoke.sh
```

Alternatively set comma-separated `TIKEO_TEST_DATABASE_URLS` to run additional endpoints.

## Configuration assets

- SQLite default: `config/dev.toml`.
- PostgreSQL example: `config/postgres.toml`.
- MySQL example: `config/mysql.toml`.
- Compose DB matrix: `deploy/compose/database-compat-compose.yml`.

## Release gate

Before marking database compatibility complete for a release branch:

1. Run `cargo fmt --all -- --check`.
2. Run `cargo test -p tikeo-storage`.
3. Run `./scripts/db-compat-smoke.sh` on a machine with Docker or equivalent external PostgreSQL/MySQL URLs.
4. Record database image/version, command log and pass/fail evidence in the release report.

If Docker or external DBs are unavailable, SQLite-only results are not sufficient to claim full SQLite/PostgreSQL/MySQL compatibility.

## Latest execution status (2026-06-01)

| Item | Command / Asset | Result | Status |
| --- | --- | --- | --- |
| SQLite compatibility smoke | `rtk cargo test -p tikeo-storage --test database_compat sqlite_database_compatibility_smoke -- --nocapture` / `rtk bash scripts/db-compat-smoke.sh` | schema bootstrap、幂等迁移、CRUD smoke 通过 | ✅ 通过 |
| PostgreSQL compatibility smoke | `rtk bash scripts/db-compat-smoke.sh` with `postgres:16-alpine` | PostgreSQL 16 迁移与 CRUD smoke 通过 | ✅ 通过 |
| MySQL compatibility smoke | `rtk bash scripts/db-compat-smoke.sh` with `mysql:8.4` | MySQL 8.4 + utf8mb4 迁移与 CRUD smoke 通过 | ✅ 通过 |
| Storage unit suite | `rtk cargo test -p tikeo-storage` | 35 passed | ✅ 通过 |
