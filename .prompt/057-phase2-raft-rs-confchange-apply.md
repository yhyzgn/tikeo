# 057 — Phase 2 raft-rs committed ConfChange application

## Context
The membership intent layer exists:
- `POST /api/v1/raft/members:propose` validates and stores `pending_conf_change` proposals only when the local node is a real Raft leader with a persisted fencing token.
- `raft_membership_proposals` is no-FK and idempotent by `(cluster_id, proposal_id)`.
- `EntryConfChange` / `EntryConfChangeV2` are still gated in apply and must not silently mutate membership.

## Required next work
1. Add a runtime command path from validated membership proposals to raft-rs `propose_conf_change` without bypassing leader/fencing checks.
2. Decode committed `EntryConfChange` / `EntryConfChangeV2` entries explicitly and apply them to raft-rs storage `ConfState` only after persistence succeeds.
3. Update `raft_membership_proposals` status from `pending_conf_change` to committed/applied/rejected based on committed entry content.
4. Update `raft_members` status only after committed ConfChange is applied; never update voters/learners from HTTP request data alone.
5. Add tests for self-removal blocking, quorum-risk blocking, committed add/remove happy paths, malformed config-change payload rejection, and replay/idempotency.
6. Preserve all hard constraints: no database foreign keys, no fake leadership, no tokenless scheduling/proposals, no Swagger UI, API envelope `{ code, message, data }`.
7. Update design/.memory/roadmap, run full verification, commit, and push.
