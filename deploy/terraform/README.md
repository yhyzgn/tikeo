# tikeo Terraform Provider

`deploy/terraform/provider` contains the in-repository Terraform provider implementation for GitOps/IaC drift review.

Implemented surfaces:

- Provider schema: `endpoint`, `api_token`, `timeout_seconds` with `TIKEO_ENDPOINT` / `TIKEO_API_TOKEN` env fallbacks.
- Data source: `tikeo_manifest` calls `GET /api/v1/gitops/manifest` and exposes checksum plus JSON/YAML manifest output.
- Resource: `tikeo_manifest_diff` stores desired manifest JSON and calls `POST /api/v1/gitops/diff`; it exposes current/desired checksum, summary JSON, and changes JSON.

The provider is intentionally review-first. It does not bulk mutate tikeo resources; approved changes should still go through typed CRUD APIs so normal RBAC, approval, audit, and validation paths remain in force.

```bash
cd deploy/terraform/provider
go test ./...
go run .
```

See `deploy/terraform/examples/manifest-diff/main.tf` for local usage.
