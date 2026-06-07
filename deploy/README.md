# Tikeo deployment 🚢

[🇨🇳 中文部署文档](../docs/zh-CN/deployment.md)

Tikeo ships deployment assets for local validation, VM/bare-metal operation, Kubernetes, and IaC/GitOps workflows.

| Path | Use it for |
| --- | --- |
| `compose/` | Docker Compose stacks for SQLite, PostgreSQL, and MySQL validation. |
| `systemd/` | Traditional server and worker units. |
| `bare-metal/` | Direct binary bootstrap and smoke checks. |
| `helm/tikeo/` | Kubernetes chart installs. |
| `k8s/operator/` | CRD/controller GitOps drift review. |
| `terraform/provider/` | Manifest export/diff provider. |
| `smoke/` | Readiness and worker bootstrap checks. |

## Operator defaults

- Keep server logs at INFO and set `observability.logging.log_dir` for durable VM/container logs.
- Prefer PostgreSQL/MySQL for shared environments.
- Do not expose business worker ports. Workers initiate outbound Worker Tunnel connections.
- Mount TLS/mTLS certificates and secret references from the deployment platform, not from Git.
