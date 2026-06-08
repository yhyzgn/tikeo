---
title: Kubernetes and Helm
description: Kubernetes, Helm, TLS, mTLS, and worker identity deployment boundaries.
---

# Kubernetes and Helm

Tikeo includes Kubernetes and Helm assets for production deployment planning.

## Helm chart baseline

The local chart under `deploy/helm/tikeo` supports:

- external database Secret injection;
- conditional SQLite PVC for development;
- HTTP listener TLS Secret mounts;
- Worker Tunnel TLS/mTLS Secret mounts;
- Ingress;
- probes, resources, security contexts;
- optional PodDisruptionBudget, NetworkPolicy, ServiceMonitor, and Gateway API GRPCRoute;
- `values.schema.json` validation.

## Worker rule

The chart must not deploy business Workers or create business Worker inbound Services. Workers connect outbound to the Worker Tunnel.

## Local validation

```bash
helm lint deploy/helm/tikeo
helm template tikeo deploy/helm/tikeo --namespace tikeo
```
