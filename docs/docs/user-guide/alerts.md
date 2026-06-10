---
title: Alerts user guide
description: Operator guide for Tikeo alert rules, alert events, silence/recovery/suppression semantics, alert delivery compatibility, and the boundary with Notification Center.
keywords: [alerts, alert rules, alert events, silence, recovery, notification center]
---

# Alerts user guide

Alerts are for abnormal conditions that need incident-like semantics. Notification Center is for reusable outbound delivery. Keep the boundary clear so normal lifecycle messages do not become noisy incidents and alert rules do not become a secret-sprawling delivery registry.

Source-backed alert surfaces live in `crates/tikeo-server/src/alert.rs`, `crates/tikeo-server/src/http/routes/alerts.rs`, `crates/tikeo-storage/src/entities/alert_rule.rs`, `crates/tikeo-storage/src/entities/alert_event.rs`, and `crates/tikeo-storage/src/repository/alert.rs`. The Notification Center migration plan is captured in `design/notification-center-alerting-plan.md`.

## Alerts vs Notification Center

| Capability | Alerts | Notification Center |
| --- | --- | --- |
| Primary job | Detect abnormal conditions and keep event history. | Deliver outbound messages to reusable destinations. |
| Core records | `alert_rules`, `alert_events`, alert-specific delivery attempts. | `notification_channels`, `notification_policies`, `notification_messages`, generic delivery attempts. |
| Semantics | Severity, rule condition, dedupe, silence, suppression, recovery, escalation intent. | Channel config, owner/event subscription, message materialization, retry, DLQ. |
| Example | Script governance failure creates a critical alert event. | Job success sends an optional Slack message. |
| Secret posture | Compatibility rules may still contain inline channels. | Reusable channels redact targets and skip `secretRefsJson` in API responses. |

Use Alerts when an operator should review an abnormal condition. Use Notification Center when a team simply wants an outbound message.

## Alert lifecycle vocabulary

Alert event statuses are represented as strings in storage and summary code:

| Status | Meaning |
| --- | --- |
| `firing` | The rule condition produced an active event that is not currently suppressed or silenced. |
| `suppressed` | The condition occurred but threshold/dedupe logic suppressed a duplicate or below-threshold event. |
| `silenced` | The rule is in a silence window and should not page operators. |
| `recovered` | A previous event was resolved/recovered. |

The event summary API counts `firing`, `suppressed`, `silenced`, and `recovered` in `AlertEventStatusCounts` and `AlertEventSummary` paths in `crates/tikeo-storage/src/repository/alert.rs`.

## Current alert API endpoints

All routes use the shared `{code,message,data}` response envelope.

| Method/path | Purpose | Permission in source |
| --- | --- | --- |
| `GET /api/v1/alert-rules` | List alert rules. | `audit:read` |
| `POST /api/v1/alert-rules` | Create an alert rule. | `audit:manage` |
| `GET /api/v1/alert-rules/{id}/delivery-status` | Validate/readiness-check inline alert channels for one rule. | `audit:read` |
| `GET /api/v1/alert-events` | List alert events with filters. | `audit:read` |
| `GET /api/v1/alert-events:summary` | Count and group alert events. | `audit:read` |
| `POST /api/v1/alert-events/{id}/resolve` | Mark an event recovered/resolved. | `audit:manage` |
| `GET /api/v1/alert-delivery-attempts` | List alert-specific delivery attempts. | `audit:read` |
| `GET /api/v1/alert-delivery-attempts:queue-status` | Alert retry/DLQ summary. | `audit:read` |
| `POST /api/v1/alert-delivery-attempts:retry-due` | Scan due alert delivery retries. | `audit:manage` |

The generic Notification Center endpoints are documented in [Notification Center reference](../reference/notification-center).

## Create an alert rule safely

Alert rules currently store `condition_json` and compatibility `channels_json`. Keep examples redacted and prefer Notification Center channels for reusable destinations.

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/alert-rules \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "script governance critical failures",
    "severity": "critical",
    "condition": {
      "resourceType": "job_instance",
      "failureClass": "script_governance_failure"
    },
    "channels": [
      {
        "type": "webhook",
        "enabled": true,
        "url": "https://hooks.example.invalid/tikeo/alerts",
        "authorization": "env:TIKEO_ALERT_SECRET_WEBHOOK_AUTH"
      }
    ],
    "enabled": true,
    "dedupeSeconds": 300
  }'
```

This example shows the compatibility shape only. Do not place raw tokens in `url`, `authorization`, `password`, `routingKey`, or similar fields. Prefer environment references and move reusable provider settings into Notification Center channels when the integration path allows it.

## Delivery readiness and queue operations

Use delivery-status before relying on an alert rule in production:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/alert-rules/alert-rule-example/delivery-status \
  -H 'Authorization: Bearer <operator-token>'
```

The response includes `ready`, `channelCount`, per-channel `provider`, `targetConfigured`, `secretConfigured`, `enabled`, `targetRedacted`, `transportSecurity`, and `issues`.

Alert retry worker defaults are separate from Notification Center defaults:

| Key | Default |
| --- | --- |
| `alert_retry.enabled` | `true` |
| `alert_retry.interval_seconds` | `60` |
| `alert_retry.batch_size` | `50` |
| `alert_retry.max_attempts` | `3` |
| `alert_retry.backoff_seconds` | `300` |

Operator retry scan:

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/alert-delivery-attempts:retry-due \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{"limit":50,"maxAttempts":3,"backoffSeconds":300}'
```

## Silence, suppression, and recovery runbook

| Situation | What it means | Operator action |
| --- | --- | --- |
| Many `suppressed` events | The condition repeated within dedupe/threshold logic. | Check whether the rule is too broad before widening delivery. |
| Events are `silenced` | `silenced_until` is still in the future. | Confirm the silence window and owner; do not bypass it with notification-only policies unless approved. |
| A condition is fixed | The event should become `recovered`. | Use the resolve endpoint or the UI/API path that records recovery evidence. |
| Alert delivery is failing | Provider target, secret, safety policy, or transport is invalid. | Check delivery-status, queue-status, then retry-due after fixing the channel. |
| A rule pages on normal success/cancel events | The rule is being used as a lifecycle notifier. | Move that use case to Notification Center. |

## UI and operational placement

The route metadata in `web/src/routes.tsx` exposes `/alerts` as **告警事件** under the observability group with `audit:read`. Treat it as the incident review surface: rule state, alert events, delivery history, and resolution evidence belong there. Treat `/notifications` as the outbound delivery center: reusable destinations, subscriptions, messages, retry queue, and DLQ inspection belong there.

## Migration guidance

The target architecture is: **Alerting produces notification messages; Notification Center owns channels and delivery**. During the compatibility phase:

- Existing `alert_rules.channels_json` and `alert_delivery_attempts` remain source-backed behavior.
- New reusable destinations should be modeled as Notification Center channels.
- Alert-specific docs should not tell operators to duplicate the same webhook token into every alert rule.
- New normal job/workflow status messages should use notification policies, not alert rules.
- Alert docs should keep incident semantics: abnormal condition, severity, dedupe, silence, suppression, recovery, and escalation.

## Troubleshooting checklist

- If alert routes return permission errors, reads require `audit:read`; rule creation, recovery, and retry scans require `audit:manage`.
- If delivery-status says `type is not registered`, use a built-in provider or an enabled plugin alert channel type.
- If `smtp://` is rejected for email, use secure SMTP outside explicit local smoke testing.
- If retry/DLQ grows, compare `alert_retry` and `notification_delivery` settings; they are separate workers and queues.
- If a page or runbook mixes “alert rule” and “notification channel” as synonyms, fix the wording before operators copy it into production procedures.
