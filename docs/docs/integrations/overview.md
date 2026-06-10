---
title: Integrations overview
description: OpenAPI, gRPC, Prometheus, OpenTelemetry, OIDC, Terraform, Kubernetes, and alert channel integration map.
---

# Integrations overview

Tikeo integrations are grouped by operational boundary.

| Integration | Purpose |
|---|---|
| HTTP API / OpenAPI | Management API and Web console contract |
| gRPC / protobuf | Worker Tunnel protocol |
| Prometheus / Grafana | Metrics, SLOs, and dashboard templates |
| OpenTelemetry | Trace export and request correlation |
| OIDC | External identity mapping into local Tikeo sessions |
| Terraform Provider | GitOps/IaC manifest and drift workflows |
| Kubernetes Operator | Tikeo manifest reconciliation and status evidence |
| Alert channels | Webhook, email, Slack, DingTalk, Feishu, WeCom, PagerDuty |

Reference automation should eventually generate API and protobuf pages from source artifacts.

## Integration design principle

Every integration should preserve Tikeo's authority boundaries. The Server owns scheduling, governance, state, APIs, and audit. Workers own execution. Deployment integrations own packaging and reconciliation. Observability integrations report evidence; they do not become the source of truth for task state.

## Current high-value integration paths

- OpenAPI and HTTP APIs for Web and SDK management clients.
- gRPC/protobuf for the Worker Tunnel protocol.
- Prometheus metrics plus Grafana dashboards for SLO visibility.
- OpenTelemetry HTTP export for traces.
- OIDC mapping into local Tikeo users, roles, and scopes.
- Terraform Provider and Kubernetes Operator assets for GitOps/IaC workflows.
- Alert delivery providers with retry/DLQ evidence.

## Documentation rule

Reference pages should eventually be generated from OpenAPI/protobuf/source artifacts. Until generation is wired, keep integration docs conceptual and link to verified commands instead of manually inventing schemas.

## Verification priority

Prioritize integrations that can be proven locally or in CI before adding long reference pages. A useful integration page should name the owning component, the source artifact, the validation command, and the failure evidence operators should expect when credentials, schemas, or network routes are wrong.
