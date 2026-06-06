# tikeo GitOps/IaC

The management plane exposes a review-first GitOps contract:

- `GET /api/v1/gitops/manifest?format=yaml|json` exports current Job, Workflow, Script, Plugin and AlertRule resources.
- `POST /api/v1/gitops/diff` accepts a desired `TikeoManifest` JSON document and returns create/update/delete/unchanged changes with unified text diffs.

Bulk apply is intentionally not implicit. Terraform and Kubernetes operator integrations use the diff endpoint and preserve typed CRUD APIs as the mutation path so normal RBAC, approval, audit and validation controls remain active.

Artifacts:

- `deploy/gitops/tikeo-manifest.example.yaml`
- `deploy/k8s/crd/tikeo-manifest-crd.yaml`
- `deploy/k8s/operator/`
- `deploy/terraform/provider/`
- `deploy/terraform/tikeo_gitops_manifest.tf`
