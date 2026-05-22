# 058 — Phase 2 raft-rs multi-node e2e validation

## Context
The raft-rs integration now has:
- Runtime ticker/inbox/outbound skeleton.
- Leader fencing lifecycle.
- State-machine command envelope and `raft_member_upsert` command.
- Membership proposal API that requires real leader/fencing.
- Committed `ConfChange` / `ConfChangeV2` decode, `RawNode::apply_conf_change`, persisted `raft_metadata.conf_state`, and `raft_members` status advancement.

## Required next work
1. Build a deterministic multi-node runtime test harness (in-process if practical) to exercise campaign/leader election without fake leadership.
2. Verify leader fencing token persistence before `can_schedule=true`.
3. Verify `POST /api/v1/raft/members:propose` succeeds only on a real leader and produces committed ConfChange in the harness.
4. Verify add/remove membership e2e updates `raft_membership_proposals`, `raft_metadata.conf_state`, and `raft_members` only after committed apply.
5. Verify outbound/inbound message flow remains Docker/K8s/LB-safe and does not require host networking.
6. Preserve all constraints: no DB foreign keys, no fake leadership, no tokenless scheduling/proposals, no Swagger UI, API envelope `{ code, message, data }`.
7. Update design/.memory/roadmap, run full verification, commit, and push.
