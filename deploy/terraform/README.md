# Tikeo Terraform Provider 🌍

[🇨🇳 中文部署文档](../../docs/zh-CN/deployment.md)

`deploy/terraform/provider` contains the in-repository provider for manifest export and drift review.

Implemented surfaces:

- Provider schema: `endpoint`, `api_token`, `timeout_seconds`.
- Data source: `tikeo_manifest`.
- Resource: `tikeo_manifest_diff`.

```bash
cd deploy/terraform/provider
go test ./...
```

The provider is review-first and does not bulk mutate resources outside Tikeo's typed API/RBAC/audit paths.
