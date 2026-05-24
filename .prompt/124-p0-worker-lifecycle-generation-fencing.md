# Phase 124 / P0 Worker lifecycle generation-fencing baseline

## Source design
Reviewed `design/worker-identity-lifecycle-design.md` before coding. This implements Slice A plus the first part of stale-message fencing.

## Goal
Keep ephemeral `worker_id` as the authoritative session id while adding logical worker grouping, generation, and fencing token semantics so restarts/reconnects do not pollute online worker state or accept stale heartbeats.

## Implementation
- Extended Worker proto: `WorkerRegistered { generation, fencing_token }`; `Heartbeat { generation, fencing_token }`.
- Server registry derives logical key from `namespace/app/cluster/region/client_instance_id`.
- Re-registering the same logical key increments generation and marks old current sessions `replaced` with reason `replaced_by_new_generation`.
- Scheduler/dispatch and `/api/v1/workers` only use latest online sessions whose lease has not expired.
- Heartbeats, task logs, and task results from stale/replaced sessions are rejected or ignored with stale-message metrics.
- Rust and Java SDKs store the server-assigned generation/fencing token and send both in heartbeats.

## Still remaining for full P0 Worker lifecycle
- Persistent `worker_logical_instances`, `worker_sessions`, `worker_session_events` tables/repositories.
- Lease scanner for `heartbeat_timeout` / `lease_expired_unknown`.
- Graceful unregister/close reason.
- Assignment token validation for logs/results.
- Web Online/Suspect/Logical/History layered UI.

## Verification
- `rtk cargo test -p tikee-server worker --all-features`
- `rtk cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`
- `rtk cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`
- `rtk bash -lc 'cd sdks/java && ./gradlew test --warning-mode all --no-daemon'`
- `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikee-help.out'`
