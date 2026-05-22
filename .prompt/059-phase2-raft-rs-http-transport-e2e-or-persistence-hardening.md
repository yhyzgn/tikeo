# 059 — Phase 2 raft-rs HTTP transport E2E / persistence hardening

## Context
Phase 2 raft-rs currently has:
- `RaftRuntimeCoordinator` ticker/inbox/outbound/apply skeleton.
- DB-backed HardState/log/snapshot/applied-command metadata foundations with no foreign keys.
- Leader fencing lifecycle: only real raft-rs leader + persisted token may set `can_schedule=true`.
- Membership proposal API -> runtime `propose_conf_change` -> committed ConfChange apply.
- Deterministic in-process 3-node RawNode harness that proves real campaign/leader election and committed membership apply without fake leadership.

## Required next work
1. Add an HTTP-level multi-node/smoke harness or equivalent integration test that exercises `/api/v1/raft/append-entries` transport DTO/auth path between runtimes without relying on host networking.
2. Harden restart/recovery semantics: initialize `MemStorage` from persisted `raft_metadata` / `raft_log_entries` where practical, or document and test the remaining gap explicitly before production enablement.
3. Verify that persisted fencing tokens are cleared or regenerated correctly after restart/non-leader observation; never reuse stale leader authority.
4. Keep raft transport Docker bridge / K8s / LB-safe (`0.0.0.0` bind, peer endpoints by service DNS/URL, optional `x-tikee-raft-token`).
5. Preserve constraints: no DB foreign keys, no Swagger UI, API envelope `{ code, message, data }`, no fake leadership, no tokenless scheduling/proposals, Go/Python SDK deferred to Phase4.
6. Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, and create the next ordered `.prompt/060-*.md`.
7. Run verification: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `cargo run -- --help`, `cd web && bun run typecheck && bun run build`.
8. Commit with rich Lore-style trailers and push to origin/main.
