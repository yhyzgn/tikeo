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
