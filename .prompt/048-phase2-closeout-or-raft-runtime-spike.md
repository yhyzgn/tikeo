# 048 — Phase 2 closeout or real Raft runtime spike

## Context
Phase2 now has dispatch_queue fencing-token plumbing in addition to ClusterCoordinator gates, Raft config/metadata/transport placeholders, diagnostics, log streaming, and workflow distributed queue foundations. Full Raft membership runtime is still intentionally incomplete; no node may set `can_schedule=true` in raft mode without real consensus.

## Goal
Decide whether to close Phase2 as “Raft runtime deferred with safe foundations” or begin a real Raft runtime spike with a bounded proof.

## Required work
1. Re-check whether `openraft` alpha status / API shape is acceptable for this project now.
2. If not acceptable, document deferral and move to the next Phase3 item (likely mTLS or audit governance) without pretending Phase2 Raft runtime is complete.
3. If acceptable, implement only a single-node real consensus smoke that can prove leadership from runtime state, not config.
4. Update design/.memory/roadmap and create `.prompt/049-*.md`.

## Constraints
- No fake leader / no raft `can_schedule=true` from config alone.
- No database foreign keys.
- Go/Python SDK remains Phase4.
- Keep API envelope `{code,message,data}`.

## Validation
- cargo fmt/clippy/test plus relevant checks.
- Commit and push with Lore trailers.
