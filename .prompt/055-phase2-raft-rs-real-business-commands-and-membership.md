# 055 — Phase 2 raft-rs real business commands and membership design

## Context
The raft-rs runtime now has safe command apply foundations:
- `raft_applied_commands` stores idempotent apply records by `(node_id, log_index)` plus `(cluster_id, command_id)`.
- `EntryNormal` payloads are parsed as a tikee command envelope: `{ "command_id": string, "command_type": string, "payload": object }`.
- `noop` is the only command type treated as applied today.
- Unknown command types are recorded as `deferred_unsupported`; invalid JSON is recorded as `rejected`; both still advance applied index deliberately.
- `EntryConfChange` / `EntryConfChangeV2` remain gated and do not silently mutate membership.
- Leader scheduling remains fenced by persisted `leader_fencing_token`.

## Required next work
1. Choose the first real business command that belongs in Raft (e.g. tikee ownership metadata, cluster membership proposal metadata, or dispatch shard lease state) and document why it is safe.
2. Implement command validation and idempotent replay semantics using `raft_applied_commands.command_id`.
3. Add tests for replay, duplicate command ids, unsupported commands, rejected payloads, and no-FK schema guarantees.
4. Design dynamic membership/config-change flow before applying any `EntryConfChange` entries.
5. Update design/.memory/roadmap, run full verification, commit, and push.

## Constraints
- Do not use database foreign keys.
- Do not enable fake leadership or tokenless scheduling.
- API envelope remains `{ code, message, data }`.
- No Swagger UI.
- Docker bridge / K8s / LB-safe networking only.
