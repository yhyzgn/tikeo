# Tikeo GitOps / IaC 🔁

[🇨🇳 中文部署文档](../../README.zh-CN.md#运行-tikeo-服务)

The management plane exposes a review-first GitOps contract:

- `GET /api/v1/gitops/manifest?format=yaml|json` exports Jobs, Workflows, Scripts, Plugins, and AlertRules.
- `POST /api/v1/gitops/diff` compares a desired `TikeoManifest` with live state.

Bulk apply is intentionally not implicit. Terraform and Kubernetes integrations use diff evidence and
preserve typed CRUD APIs as the mutation path.
