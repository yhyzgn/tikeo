# Security Policy Center Plan

Date: 2026-06-17
Status: Phase A closed; Phase B+ planned

## 1. Why this exists

Tikeo already has several real security enforcement points:

- RBAC resources/actions and scoped API keys.
- Script execution policy snapshots: timeout, memory, output size, network, filesystem, secret refs, env vars, sandbox backend.
- Script publish/rollback gates: approval ticket, signature verification, release grant evidence, audit failure reasons.
- Worker runtime grant enforcement: signed URL/File/Secret grants are copied to Worker Tunnel script bindings and fail closed when a runner cannot safely enforce them.
- Notification Center redaction and channel test-send safety policies.
- Raft transport token and Worker Tunnel TLS/mTLS deployment surfaces.

The missing product surface is **not** another hidden policy engine. The missing surface is a unified operator-facing center that answers:

1. Which policies exist and where are they enforced?
2. Which dangerous capabilities are currently blocked, locally signed, or externally verified?
3. Which scripts/jobs/workers are affected by a policy change?
4. Which audit events prove a release was allowed or denied?
5. Which deployment/network policies are still external prerequisites?

## 2. Product name and boundary

Name: **Security Policy Center**.

It is a governance console and API layer over existing enforcement points first. OPA/Rego or a custom DSL can be added later, but v1 should not wait for a new language if existing Tikeo policies already enforce important boundaries.

Non-goals for v1:

- Do not execute user-supplied Rego/DSL inside the Server.
- Do not replace RBAC, script release gates, or Worker runtime enforcement.
- Do not store provider secrets in policy rows.
- Do not enable network/file/secret grants unless release evidence proves runner-side enforcement.
- Do not expose future policy/evaluation ledger features as complete until API, UI, RBAC, audit, and tests are in place.

## 3. Current evidence-backed enforcement points

| Enforcement point | Current status | Evidence anchors |
| --- | --- | --- |
| Script policy default deny | Implemented | `ScriptExecutionPolicy::validate_default_deny`, `crates/tikeo-server/src/http/routes/scripts.rs`, HTTP tests for dangerous network policy rejection. |
| Signed release metadata | Implemented | Script publish/rollback routes, `script_release_signature`, audit failure reasons. |
| URL/File/Secret grants | Implemented as signed evidence + fail-closed runtime boundaries | `ScriptReleaseGrantSet`, `dispatch_copies_verified_release_grants_into_script_binding`, Rust SDK grant enforcement tests. |
| RBAC and scoped API keys | Implemented | HTTP auth scope validation, RBAC seed, roles/users/API key pages. |
| Notification redaction | Implemented | Notification repository redaction, provider-specific test-send behavior. |
| Cluster transport token | Implemented | Raft transport token docs/config, HA runbook. |
| Unified policy center API/UI | Phase A implemented | `/api/v1/security/posture`, `/api/v1/security/transport`, Web `/security`, RBAC `security:read`, and docs are source-backed; managed policy ledger remains Phase B. |

## 4. Data model proposal

Use explicit rows rather than free-form blobs as the authoritative v1 shape:

```text
security_policies
- id
- name
- policy_type: script_execution | release_gate | network_egress | file_access | secret_access | notification_delivery | cluster_transport
- scope_type: global | namespace | app | worker_pool | script | job
- scope_id
- mode: observe | enforce | disabled
- status: active | draft | archived
- rule_json
- created_by / updated_by / timestamps

security_policy_evaluations
- id
- policy_id
- resource_type / resource_id
- decision: allowed | denied | observed
- reason_code
- evidence_json
- evaluated_at

security_policy_bindings
- id
- policy_id
- target_type: namespace | app | worker_pool | script | job | notification_channel | cluster
- target_id
- priority
- enabled
```

The first implementation may project from existing script policies and audit logs instead of immediately migrating all policy truth into these tables. If projection is used, the API must label each item as `source=script_policy_snapshot|audit_log|rbac|config`.

## 5. API proposal

```text
GET  /api/v1/security/policies
GET  /api/v1/security/policies/{id}
POST /api/v1/security/policies:validate
GET  /api/v1/security/evaluations?resourceType=&resourceId=&decision=
GET  /api/v1/security/posture
GET  /api/v1/security/affected-resources?policyId=
```

RBAC:

| Operation | Permission |
| --- | --- |
| Read policy posture | `security:read` |
| Validate draft policy | `security:read` + target resource read |
| Create/update/archive policy | `security:manage` |
| View evaluations/audit evidence | `security:read` + target resource scope check |

## 6. UI proposal

Route: `/security` under Governance.

Tabs:

1. **Posture**: current default-deny status, unsigned grant count, policy warning count, TLS/mTLS/transport-token checks.
2. **Policies**: table by policy type/scope/mode/status/source.
3. **Evaluations**: recent allow/deny/observe decisions with links to audit logs, scripts, jobs, notification messages, or cluster diagnostics.
4. **Affected resources**: scripts/jobs/workers/channels impacted by a selected policy.
5. **Deployment checks**: network/TLS/secret requirements that are outside the Server and must be proven by runbooks.

The previous placeholder/disabled route has been replaced by the source-backed `/security` posture page. Policies/Evaluations ledger tabs remain Phase B+ scope and must not be presented as implemented until the ledger exists.

## 7. Implementation phases

### Phase A — posture projection (closed)

- ✅ Added `security:read` / `security:manage` permissions to RBAC seed and route metadata.
- ✅ Added read-only posture API derived from existing sources:
  - script policy snapshots;
  - script release grants;
  - audit failure reasons;
  - notification channel redaction/test-send safety;
  - transport security config presence;
  - cluster transport token presence.
- ✅ Added Web page `/security` with real read-only data and moved the menu under Governance.
- ✅ Added HTTP route/OpenAPI, Web source tests, docs page, and docs sidebar/index coverage.

Evidence anchors after closure:

- API/router: `crates/tikeo-server/src/http/routes/security.rs`, `crates/tikeo-server/src/http/router.rs`, `crates/tikeo-server/src/http/openapi.rs`.
- RBAC/menu: `crates/tikeo-server/src/http/routes/roles.rs`, `web/src/routes.tsx`, `web/src/App.tsx`.
- Web: `web/src/pages/SecurityPolicyCenterPage.tsx`, `web/src/api/security.ts`, `web/src/pages/__tests__/SecurityPolicyCenterPage.test.ts`.
- Docs: `docs/docs/user-guide/security-policy-center.md`, `docs/i18n/zh-CN/docusaurus-plugin-content-docs/current/user-guide/security-policy-center.md`.

### Phase B — policy/evaluation ledger

- Add explicit SeaORM migration for `security_policies`, `security_policy_bindings`, `security_policy_evaluations`.
- Materialize script publish/rollback and dispatch policy decisions into evaluations.
- Add filtering, pagination, and audit links.
- Tests: migration tests, repository tests, HTTP tests, audit/evaluation consistency tests.

### Phase C — enforced managed policies

- Allow managed URL/File/Secret policy objects to be bound to scripts/jobs/worker pools.
- Keep fail-closed if Worker runtime cannot enforce a granted capability.
- Optional: add external verifier adapters for Vault/KMS/PKI.

### Phase D — optional DSL / OPA integration

- Add DSL/Rego only after Phase A-C establish data and evaluation boundaries.
- Sandbox/timeout every evaluation.
- Never let DSL policy bypass RBAC, release signing, or runner-side grant enforcement.

## 8. Acceptance checklist

- [x] No disabled or placeholder Security menu item is presented as complete.
- [x] `/api/v1/security/posture` returns only source-backed data.
- [x] Web page renders posture from API, not hardcoded examples.
- [x] Script dangerous policy/grant denial shows up in posture/recent denials where existing audit evidence is available; full materialized evaluations remain Phase B.
- [x] RBAC requires `security:read` for the route/API surface; focused tests should be run before each release.
- [ ] Full policy/evaluation ledger links every policy change and decision to actors/resources. This is Phase B scope, not Phase A.
- [x] Docs explain which checks are Server-enforced and which are deployment prerequisites.
- [x] Tests cover route/source shape, docs contracts, and source-size limits; release verification should additionally run focused server/Web checks.
