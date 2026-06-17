---
title: Security Policy Center
description: Operator guide for source-backed security posture, policy evidence, and deployment prerequisites in Tikeo.
---

# Security Policy Center

Security Policy Center is the operator-facing view of security controls that already enforce behavior in Tikeo. It is not a mock policy lab and it does not execute a user-supplied DSL. The page reads source-backed posture from `/api/v1/security/posture` and shows evidence from configuration, script policy snapshots, notification channel redaction metadata, cluster transport settings, and audit logs.

## What it answers

| Question | Source used by the page |
| --- | --- |
| Are script execution policies still default-deny? | Stored `ScriptExecutionPolicy` snapshots on script rows. |
| Are dangerous script capabilities blocked? | Server-side create/update validation and release-gate audit failures. |
| Is script release signing configured? | `script_governance.release_signature_secret_ref`. |
| Are notification targets redacted? | `notification_channels.target_redacted`, redacted config, and safety policy metadata. |
| Is transport ready for trusted deployments? | HTTP/Worker Tunnel TLS/mTLS status and Raft transport-token presence. |
| What was recently denied? | Failed audit events, especially script publish/release-gate denials. |

## Required permission

The menu entry and API require `security:read`. The built-in owner, operator, and viewer roles receive read access through the Security Policy Center RBAC migration. `security:manage` is reserved for later managed policy phases and is only seeded for owner.

## Posture model

`GET /api/v1/security/posture` returns:

- `overallStatus`: `ok`, `warning`, or `critical` derived from checks.
- `checks`: each check has `id`, `status`, `source`, `detail`, and `evidenceCount`.
- `scriptGovernance`: counts scripts that validate as default-deny, dangerous snapshots, released scripts, signed releases, and grant-backed releases.
- `notificationSafety`: counts configured/redacted targets and safety-policy metadata.
- `clusterTransport`: reports Raft token and TLS readiness booleans.
- `recentDenials`: recent failed policy/audit events, including script release gate denials.

## Interpreting statuses

| Status | Meaning | Typical action |
| --- | --- | --- |
| `ok` | The check has source-backed evidence and no local issue. | Keep existing rollout evidence. |
| `warning` | The setup may be valid in dev, but production needs a stronger deployment prerequisite. | Review TLS/mTLS, Raft token, release-signing, or network-layer configuration. |
| `critical` | Persisted policy evidence shows an unsafe state. | Stop rollout, inspect the affected script/channel/config, and use audit logs to identify the change. |

## Deployment prerequisites shown on the page

Security Policy Center can confirm process-level settings, but it cannot prove every external network layer property by itself. For production, combine it with:

- [Production deployment](../deployment/production)
- [Server HA and Raft FSOD Cluster](../deployment/server-ha)
- [SSE realtime channel deployment](../deployment/sse-realtime)
- [Configuration reference](../reference/configuration)

In particular, cloud LB/WAF/TLS/multi-zone behavior still requires the environment-specific HA validation that is intentionally separate from the local Kind evidence.

## API smoke check

```bash
curl -fsS \
  -H "Authorization: Bearer $TIKEO_TOKEN" \
  http://127.0.0.1:9090/api/v1/security/posture | jq '.data.overallStatus, .data.checks[] | {id,status,source}'
```

A healthy production candidate should have no `critical` checks. Warnings are acceptable only when they are explicitly explained by the target deployment model, for example TLS terminated at an ingress plus an internal mTLS plan documented elsewhere.
