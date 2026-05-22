# 041 — Phase 2 server cluster / Raft planning

## Context
Phase 2 storage targets now include SQLite, MySQL, PostgreSQL, and CockroachDB at the SeaORM/sqlx feature level. Go/Python SDKs are deferred to Phase 4 per user instruction.

## Goal
Plan the Server cluster (Raft consensus) work item without prematurely implementing a partial unsafe cluster mode.

## Required work
1. Review existing dispatch_queue lease/claim behavior and identify what still requires consensus vs database conditional updates.
2. Define the Raft responsibilities: tikee leadership, tick ownership, dispatcher ownership, config changes, and failover semantics.
3. Decide crate boundaries under `crates/` for cluster coordination.
4. Produce/update design docs and route-map tasks for implementation.
5. If a minimal safe implementation slice is obvious, implement only that slice with tests.

## Constraints
- Do not reintroduce DB foreign keys.
- Do not use Go/Python SDK work in Phase 2.
- Server and Worker must remain container/K8s deployable across bridge/LB/WAF networks.
- Keep API envelope `{code,message,data}` unchanged.

## Validation
- Run cargo fmt/clippy/test for changed Rust code.
- Update `.memory` and roadmap.
- Commit and push with Lore trailers.
