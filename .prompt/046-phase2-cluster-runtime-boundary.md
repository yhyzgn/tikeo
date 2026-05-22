# 046 — Phase 2 cluster runtime boundary and health visibility

## Context
Raft config, metadata persistence, fencing-token shape, and a reserved HTTP AppendEntries transport endpoint now exist. The coordinator is still deliberately storage-backed/no-op in `tikee-server::cluster`; raft mode remains `role=unknown`, `leader_fencing_token=null`, and `can_schedule=false` until a real consensus runtime produces leadership.

## Goal
Improve cluster runtime readiness without fake leader behavior.

## Required work
1. Add operator-visible cluster diagnostics for configured peers, metadata term/index, transport placeholder status, and whether scheduling is gated.
2. Decide whether this diagnostic surface belongs under `/api/v1/cluster` or a new `/api/v1/cluster/diagnostics` endpoint.
3. Keep `ClusterCoordinator` as the only scheduling ownership source.
4. Do not create `tikee-cluster` crate unless there is a clear stable boundary; document the decision.
5. Update design/.memory/roadmap.

## Constraints
- No fake leader and no raft `can_schedule=true` without consensus.
- No database foreign keys.
- Go/Python SDK remains Phase4.
- Container/K8s/LB networking remains the deployment assumption.

## Validation
- cargo fmt/clippy/test.
- Commit and push with Lore trailers.
