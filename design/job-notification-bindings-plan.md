# Job Notification Bindings and Execution Log Passthrough Plan

Date: 2026-06-13
Status: Accepted for implementation

## 1. Problem

Tikeo already has a reusable Notification Center with channels, templates, policies, messages, delivery attempts, retry, DLQ, provider schemas, and channel test-send. Operators can configure channels independently from jobs. That separation is correct for credentials and delivery governance, but it is not enough for day-to-day job operations: a job owner needs an obvious way to say "when this job succeeds, fails, or always finishes, notify these channels with this template and include instance context/log evidence".

The feature must not duplicate Notification Center. It should add a job-scoped binding layer that is easy to configure from the job detail/edit UI while still materializing through Notification Center messages and attempts.

## 2. Vocabulary and boundaries

- **Notification Channel**: reusable provider target and credentials. Owned by Notification Center.
- **Notification Template**: reusable provider-aware message shape. Owned by Notification Center.
- **Notification Policy**: normalized routing rule that turns domain events into notification messages. Owned by Notification Center.
- **Job Notification Binding**: job-owner-facing configuration that compiles to one or more job-scoped notification policies.
- **Notification Message Detail**: an operator/developer trace page for one message, including event context, policy/template/channel resolution, delivery attempts, and a safe link/excerpt for the related job instance logs.
- **Execution Log Passthrough**: UI/API affordance that displays logs for the related instance from the notification trace without exposing Worker internals or provider secrets.

Alerting remains the abnormal-condition incident subsystem. Job Notification Bindings are not Alert Rules. They are normal job lifecycle notification routing.

## 3. Goals

1. Configure notifications from a job context without re-entering provider credentials.
2. Support trigger presets: `success`, `failure`, `always`, plus explicit advanced states.
3. Carry rich instance context into templates: job id/name/scope, instance id/status/timing/attempt/trigger/execution mode/operator/worker/log URL/log excerpt.
4. Provide a message detail and log passthrough page for developers to debug what notification was emitted and how it was delivered.
5. Preserve Notification Center retry/DLQ, redaction, RBAC, audit, and provider adapter behavior.
6. Keep all source files under 1500 lines and keep schema changes explicit.
7. Ensure the `tikeo` binary/API version reflects the released tag; this is now covered by `scripts/set-release-version.py` for release workflows and verified by `.github/tests/release_version_script_test.py`.

## 4. Non-goals

- Do not store raw provider tokens/passwords on jobs.
- Do not add a second delivery engine.
- Do not make business Workers expose inbound ports.
- Do not embed full unbounded logs into provider payloads. Use links and bounded redacted excerpts only.
- Do not claim live SaaS smoke without credentials.

## 5. Data model

### 5.1 Preferred storage representation

Reuse `notification_policies` as the durable routing table and add a first-class job-facing API layer over it.

A job binding is represented by one policy row:

```text
notification_policies.owner_type = 'job'
notification_policies.owner_id   = jobs.id
notification_policies.event_family = 'job_instance'
notification_policies.event_types_json = [expanded event types]
notification_policies.channel_refs_json = [{ channelId }]
notification_policies.template_ref = optional template id/key
notification_policies.filters_json = {
  "jobId": "...",
  "triggerPreset": "success|failure|always|advanced",
  "includeLogLink": true,
  "includeLogExcerpt": false,
  "logExcerptLines": 80,
  "notifyOnRetryScheduled": false
}
```

This avoids duplicating channel/template references while giving Jobs a friendly API.

### 5.2 Trigger preset expansion

| Binding trigger | Event types |
| --- | --- |
| `success` | `job_instance.succeeded` |
| `failure` | `job_instance.failed`, `job_instance.partial_failed`, `job_instance.retry_exhausted`, `job_instance.no_eligible_worker`, `job_instance.script_governance_failure` |
| `always` | all terminal events: success + failure group + `job_instance.cancelled` |
| `cancelled` | `job_instance.cancelled` |
| `retry_scheduled` | `job_instance.retry_scheduled` |
| `retry_exhausted` | `job_instance.retry_exhausted` |
| `advanced` | caller-supplied allow-listed event types |

Retry semantics must stay precise: while another retry is scheduled, emit `retry_scheduled`; final failure is emitted only when retries are exhausted or no retry remains.

## 6. API design

Add job-scoped convenience endpoints:

```http
GET    /api/v1/jobs/{job_id}/notification-bindings
POST   /api/v1/jobs/{job_id}/notification-bindings
GET    /api/v1/jobs/{job_id}/notification-bindings/{binding_id}
PATCH  /api/v1/jobs/{job_id}/notification-bindings/{binding_id}
DELETE /api/v1/jobs/{job_id}/notification-bindings/{binding_id}
POST   /api/v1/jobs/{job_id}/notification-bindings:validate
POST   /api/v1/jobs/{job_id}/notification-bindings:preview
```

Add message trace endpoint:

```http
GET /api/v1/notification-messages/{message_id}/trace
```

Trace response shape:

```json
{
  "message": { "id": "...", "status": "...", "eventType": "..." },
  "policy": { "id": "...", "ownerType": "job", "ownerId": "...", "templateRef": "..." },
  "job": { "id": "...", "name": "...", "namespace": "...", "app": "..." },
  "instance": { "id": "...", "status": "...", "startedAt": "...", "finishedAt": "...", "triggerType": "...", "executionMode": "..." },
  "operator": { "type": "human|api_key|system", "name": "..." },
  "logs": { "url": "/instances/.../logs", "excerpt": "...", "truncated": true },
  "attempts": [{ "channelId": "...", "provider": "...", "delivered": true, "retryState": "delivered", "targetRedacted": "..." }]
}
```

All APIs keep the existing `{code,message,data}` envelope.

## 7. Template context

Job lifecycle notification template context should include:

```json
{
  "job": {
    "id": "job id",
    "name": "job name",
    "namespace": "namespace",
    "app": "app",
    "executionMode": "single|broadcast|shard|workflow"
  },
  "instance": {
    "id": "instance id",
    "status": "failed",
    "attempt": 1,
    "startedAt": "RFC3339",
    "finishedAt": "RFC3339",
    "durationMs": 1234,
    "triggerType": "api|cron|manual|workflow|system"
  },
  "operator": {
    "type": "human|api_key|system",
    "id": "principal id if available",
    "name": "display name or service account"
  },
  "worker": {
    "id": "worker id if known",
    "pool": "worker pool if known"
  },
  "logs": {
    "url": "/instances/{id}/logs",
    "excerpt": "bounded redacted excerpt if enabled"
  }
}
```

`logs.excerpt` is bounded and redacted. Full logs stay in the existing instance log storage/API.

## 8. Web UX

### 8.1 Job notification configuration

Add a `Notifications` area in Job detail/edit flows:

- Empty state: explain that channels/templates are reusable and credentials live in Notification Center.
- Binding list: trigger, channels, template, enabled, include log link/excerpt, last updated.
- Drawer form:
  - trigger preset selector with advanced event checklist;
  - channel multi-select filtered to enabled Notification Center channels;
  - template selector filtered by selected providers;
  - include log link switch;
  - include log excerpt switch + line limit;
  - validate/preview buttons;
  - save.

### 8.2 Message detail / log passthrough

Add route:

```text
/notifications/messages/:messageId
```

Page sections:

1. Event overview: status, job, instance, trigger, operator, timing.
2. Delivery timeline: one row per attempt/channel with redacted provider target and retry/DLQ state.
3. Rendered payload summary: provider-safe redacted preview.
4. Execution logs: bounded excerpt and link to full instance logs page/API.

Design should be production admin-console style: clear status colors, compact timeline, copyable ids, strong empty/error states, no secret display.

## 9. RBAC and audit

- Read bindings: existing job read plus notifications read where necessary.
- Manage bindings: job update permission and notification policy manage capability.
- Message trace: notification read plus instance/job read within scope.
- Test/preview: no provider delivery unless explicitly invoking channel test-send; preview only renders template context.
- Audit binding create/update/delete as `job_notification_binding` actions.

## 10. Versioning fix

Release builds now synchronize workspace/package metadata from the tag before compiling. Keep build-time version metadata resolving in this order:

1. `TIKEO_VERSION` environment override from GitHub Actions release workflows.
2. `GITHUB_REF_NAME` when it is a `v*` tag.
3. `CARGO_PKG_VERSION` plus short git commit for non-release builds.

CI/release workflows must synchronize the workspace from the tag before server builds so `tikeo --version`, `/api/v1/system/info`, and release artifacts agree with the tag. This sync updates Cargo workspace manifests and the local workspace package entries in `Cargo.lock`; release builds must keep `--locked` enabled so dependency drift is still rejected.

## 11. Acceptance criteria

- Source-size gate passes.
- Job owner can configure success/failure/always notifications from the Job UI.
- Runtime job instance transitions create Notification Center messages/attempts through reusable channels/templates.
- Template render context includes job/instance/operator/timing/log fields.
- Message detail page shows event context, delivery attempts, redacted provider output, and execution log passthrough.
- Disabled/stale channels/templates fail closed in validation and runtime.
- Secrets never appear in API responses, UI, docs, logs, or tests.
- `tikeo` release version reflects the pushed `v0.2.x` tag.
- Local verification and GitHub Actions CI/Coverage/Release/Docker/SDK workflows pass for the final release tag.
