# Tikeo deployment 🚢

[🇨🇳 中文部署文档](../README.zh-CN.md#运行-tikeo-服务)

Tikeo ships deployment assets for local validation, VM/bare-metal operation, Kubernetes, and IaC/GitOps workflows.

| Path | Use it for |
| --- | --- |
| `compose/` | Docker Compose stacks for SQLite, PostgreSQL, and MySQL validation. |
| `systemd/` | Traditional server and worker units. |
| `bare-metal/` | Direct binary bootstrap and smoke checks. |
| `helm/tikeo/` | Kubernetes chart installs, external DB secret wiring, TLS/mTLS values, rollback runbooks, PDB, NetworkPolicy, ServiceMonitor, and Gateway API examples. |
| `k8s/operator/` | CRD/controller GitOps drift review. |
| `terraform/provider/` | Manifest export/diff provider. |
| `smoke/` | Readiness and worker bootstrap checks. |

## Operator defaults

- Keep server logs at INFO and set `observability.logging.log_dir` for durable VM/container logs.
- Prefer PostgreSQL/MySQL/CockroachDB for shared environments and inject database URLs through platform Secrets.
- Do not expose business worker ports. Workers initiate outbound Worker Tunnel connections.
- Mount TLS/mTLS certificates and secret references from the deployment platform, not from Git.
- Helm production overlays live under `helm/tikeo/examples/` and include external database, listener TLS/mTLS, ingress TLS, worker identity, PDB, NetworkPolicy, ServiceMonitor, Gateway API, and rollback guidance.
