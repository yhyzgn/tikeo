---
title: Integrations overview
description: Operator map for Tikeo HTTP, Worker Tunnel, inbound event, outbound notification, observability, identity, and deployment integrations.
---

# Integrations overview

Use this page to decide which Tikeo integration surface to operate, which component owns it, and which command proves the path is reachable. Keep inbound event triggers separate from outbound notification channels: they solve opposite traffic directions and use different routes.

## Integration map

| Integration surface | Direction | Owning component | Primary contract tokens | Verification entry point |
| --- | --- | --- | --- | --- |
| HTTP Management API / OpenAPI | Operator or SDK to Server | Server | `/api-docs/openapi.json`, `/api/v1/jobs`, `/api/v1/workers` | `curl http://127.0.0.1:9090/api-docs/openapi.json` |
| Worker Tunnel | Worker to Server | Worker SDKs + Server | Worker Tunnel listener `:9998`, `WorkerTunnelService`, `OpenTunnel`, `DispatchTask`, `TaskLog`, `TaskResult` | Run a demo worker and check `/api/v1/workers` |
| Inbound webhook triggers | External system to Server | Server event-source route | `POST /api/v1/events/webhooks/{job}:trigger` | Trigger a known job and inspect instance logs |
| Outbound Notification Center | Server to external provider | Notification Center | `/api/v1/notification-channels`, `/api/v1/notification-policies`, `/api/v1/notification-delivery-attempts` | Create a channel with `secretRefs`, then validate a policy |
| Alerts | Server to incident workflow | Alerting + optional delivery | `/api/v1/alert-rules`, `/api/v1/alert-events`, `/api/v1/alert-delivery-attempts` | Create/read alert rules and delivery status |
| Prometheus metrics | Scraper to Server | Server observability | `/metrics` | `curl http://127.0.0.1:9090/metrics` |
| OpenTelemetry traces | Server to collector | Server observability | `observability.tracing.enabled`, `observability.tracing.otlp_endpoint` | Check Server logs and collector intake |
| OIDC login | Browser/API to identity provider and Server | Server auth | `auth.oidc.*`, `/api/v1/auth/oidc/*` | Check bootstrap/login and OIDC callback behavior |
| Terraform / GitOps | IaC runner to Server | Deploy tooling + Server GitOps API | `deploy/terraform/`, `/api/v1/gitops/manifest`, `/api/v1/gitops/diff` | `deploy/smoke/terraform-provider-smoke.sh` |
| Kubernetes / Helm | Cluster controller to workloads | Deploy assets | `deploy/helm/tikeo/`, `deploy/k8s/`, `TikeoManifest` CRD | `helm template` or k8s dry-run smoke |

## Traffic direction rules

### Inbound event triggers

Use inbound triggers when an external event should start a Tikeo job. The route is:

```text
POST /api/v1/events/webhooks/{job}:trigger
```

The request body accepts these fields:

- `source`
- `eventType`
- `payload`
- `signature`
- `timestamp`
- `nonce`
- `secretRef`

The same values can also be supplied through headers where supported:

- `x-tikeo-webhook-secret-ref`
- `x-tikeo-webhook-signature`
- `x-tikeo-webhook-timestamp`
- `x-tikeo-webhook-nonce`

If signature fields are present, Tikeo validates timestamp freshness, nonce replay, and a `secretRef` resolved from the Server environment. Keep signing material in environment variables, not request examples or docs.

Minimal local shape for a trusted disposable test job:

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/events/webhooks/${JOB_ID}:trigger \
  -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"source":"local-smoke","eventType":"demo.event","payload":{"example":true}}' | jq .
```

This creates a job instance with `triggerType=webhook` and appends an instance log containing `webhook_event_source`.

### Outbound notification channels

Use Notification Center when Tikeo must send messages to Slack, DingTalk, Feishu/Lark, WeCom, PagerDuty, email, generic webhooks, or plugin webhook-compatible providers.

Key routes:

```text
GET  /api/v1/notification-channel-types
POST /api/v1/notification-channels
POST /api/v1/notification-policies
GET  /api/v1/notification-delivery-attempts
GET  /api/v1/notification-delivery-attempts:queue-status
POST /api/v1/notification-delivery-attempts:retry-due
```

Store provider targets and credentials in each channel row's `secretRefs`, for example `env:TIKEO_NOTIFICATION_CHANNEL_BILLING_FEISHU_WEBHOOK_URL`, `env:TIKEO_NOTIFICATION_CHANNEL_BILLING_FEISHU_SIGNING_KEY`, or `env:TIKEO_NOTIFICATION_CHANNEL_ONCALL_PAGERDUTY_ROUTING_KEY`. Use different refs for different Slack/DingTalk/Feishu/WeCom/PagerDuty/email/webhook channels; do not store provider tokens inside examples, templates, tickets, screenshots, or channel `config` JSON.

## Step-by-step local integration checks

### 1. Verify the API and OpenAPI surface

```bash
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
python3 -m json.tool /tmp/tikeo-openapi.json >/dev/null
```

If the OpenAPI document is unavailable, fix Server startup before debugging SDKs or UI calls.

### 2. Verify Worker Tunnel connectivity

Use one of the maintained demo workers. For the Node.js demo path, follow [Quickstart](../getting-started/quickstart). Then check:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/workers \
  -H "authorization: Bearer $TOKEN" \
  | jq '.data.items[] | {clientInstanceId,status,namespace,app,structuredCapabilities}'
```

A worker that is missing from this list is not connected to the Server, even if its local process is running.

### 3. Verify inbound webhook trigger routing

Create or choose an API-scheduled job, export its ID as `JOB_ID`, then call:

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/events/webhooks/${JOB_ID}:trigger \
  -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"source":"local-smoke","eventType":"demo.event","payload":{"orderId":"demo-1"}}' | jq .data
```

Verify the instance:

```bash
curl -fsS 'http://127.0.0.1:9090/api/v1/instances?page=1&pageSize=20' \
  -H "authorization: Bearer $TOKEN" | jq '.data.items[0]'
```

If a signature is required by your operating procedure, generate it outside the docs and pass only a `secretRef`, signature, timestamp, and nonce. Never write signing material into the request body or command history.

### 4. Verify outbound notification metadata

```bash
curl -fsS http://127.0.0.1:9090/api/v1/notification-channel-types \
  -H "authorization: Bearer $TOKEN" \
  | jq '.data[] | {type, label, secretFields}'
```

This should list built-in providers such as `webhook`, `slack`, `dingtalk`, `feishu`, `wechat_work`, `pagerduty`, and `email`, plus enabled plugin channel types if configured.

### 5. Verify deployment integration assets

Use local render/dry-run checks before live cluster or IaC changes:

```bash
helm template tikeo deploy/helm/tikeo >/tmp/tikeo-helm.yaml
kubectl apply --dry-run=client -f deploy/k8s/tikeo.yaml
scripts/db-seed-api-compat-smoke.sh
```

For Terraform provider checks, start with:

```bash
deploy/smoke/terraform-provider-smoke.sh
```

Run live Terraform or Kubernetes smoke tests only against an environment you are allowed to modify.

## Troubleshooting by integration

| Symptom | Likely boundary | Operator check |
| --- | --- | --- |
| SDK calls return unauthorized | HTTP API auth | Bearer token versus `x-tikeo-api-key`, scopes, namespace/app binding. |
| Worker process is running but no jobs execute | Worker Tunnel | Endpoint `http://127.0.0.1:9998`, TLS/plaintext match, namespace/app/worker pool, processor names. |
| Webhook trigger returns replay or signature errors | Inbound trigger | Timestamp within 300 seconds, unique nonce, resolvable `secretRef`, matching payload when the signature was generated. |
| Notifications do not leave Tikeo | Outbound channel | Channel enabled, `secretRefs` resolve, provider URL reachable, delivery queue status and retry/DLQ state. |
| Alert rule pages for normal lifecycle messages | Alerting versus notifications | Move normal lifecycle messages to Notification Center policies; keep Alerts for abnormal conditions. |
| Metrics scrape fails | Prometheus | Server reachable, `/metrics` path, network policy, service monitor target. |
| OTel export has no traces | OpenTelemetry | `observability.tracing.enabled`, collector endpoint, headers, Server logs. |
| Helm works but pods are not ready | Deployment | ConfigMap values, DB secret wiring, probes against `/readyz`, Worker Tunnel service exposure. |

## Production checklist

- Keep inbound webhook trigger credentials separate from outbound provider credentials.
- Use `secretRefs` and environment-backed secrets for channels and signed webhook validation.
- Document the owning team for each integration surface.
- Record a verification command and expected failure mode for every enabled integration.
- Use `/readyz` or `/healthz` for health checks; do not probe stream endpoints or job trigger routes.
- Run local render/dry-run checks before applying Terraform, Helm, or Kubernetes changes to a shared environment.

## Prerequisites

Use the setup, authentication, and access requirements described in this page before running any command. For local examples, start the Server with `config/dev.toml`, use `127.0.0.1` as the client host, and keep tokens in shell variables rather than pasted into files.

## Verify

After following the page, verify the result with the documented API, UI, build, smoke, or deployment checks. A valid verification includes the command that was run, the route or file that was inspected, and the observed status or artifact.
