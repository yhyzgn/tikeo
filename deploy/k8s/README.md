# Kubernetes deployment ☸️

[🇨🇳 中文部署文档](../../docs/zh-CN/deployment.md)

Tikeo runs server and web as separate workloads. Workers should run in business namespaces or
clusters and initiate outbound gRPC connections to the Worker Tunnel endpoint.

## CRD/operator

- `deploy/k8s/crd/tikeo-manifest-crd.yaml` defines the namespaced `TikeoManifest` CRD.
- `deploy/k8s/operator/` reconciles desired manifests through the GitOps diff endpoint.
- `applyMode=diffOnly` is the safe default; typed CRUD APIs remain the mutation path so RBAC, audit,
  approval, and validation are not bypassed.
