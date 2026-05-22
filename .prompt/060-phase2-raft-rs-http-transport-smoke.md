# 060 — Phase 2 raft-rs HTTP transport smoke / Docker bridge E2E

## Context
Current raft-rs Phase2 state:
- Runtime Ready processing persists HardState/log/snapshot/apply state and keeps `MemStorage` aligned before `advance_append`.
- Runtime startup restores HardState and persisted log entries from `raft_metadata` / `raft_log_entries` and clears stale `leader_fencing_token` until a real leader role is observed again.
- In-process RawNode harness verifies real campaign/election, fencing persistence, and committed membership apply.
- Remaining transport confidence gap: outbound/inbound raft messages have DTO/auth/unit coverage but not a multi-runtime HTTP smoke over container-like networking.

## Required next work
1. Build an HTTP-level smoke/integration harness that exercises `/api/v1/raft/append-entries` with `x-tikee-raft-token` between server/runtime instances or a realistic local equivalent, without host-network assumptions.
2. Prefer Docker bridge or service-name style endpoints where practical; if the repo test environment cannot spin containers reliably, add a documented local HTTP test that proves route/auth/envelope/DTO conversion and runtime inbox delivery, then keep Docker bridge as an explicit manual verification script.
3. Ensure every response uses `{ code, message, data }`, and `accepted=true` remains only “queued by local runtime”, not leadership/scheduling authority.
4. Preserve constraints: no DB foreign keys, no Swagger UI, no fake leadership, no stale fencing reuse, `0.0.0.0`/9998 defaults, Go/Python SDK deferred to Phase4.
5. Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, and create `.prompt/061-*.md` for the next Phase2 item.
6. Run verification: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `cargo run -- --help`, `cd web && bun run typecheck && bun run build`.
7. Commit with Lore-style trailers and push to origin/main.
