# tikeo K8s CRD controller/operator

This directory contains the in-repository operator implementation for `TikeoManifest` resources.

Implemented surfaces:

- CRD status subresource in `deploy/k8s/crd/tikeo-manifest-crd.yaml`.
- Reconciler package that validates `spec.applyMode`, posts `spec.manifest` to `/api/v1/gitops/diff`, and builds status fields: `observedGeneration`, `checksum`, `currentChecksum`, `desiredChecksum`, `summary`, `lastDiff`, and `conditions`.
- Operator CLI entrypoint with `--kubeconfig`, `--tikeo-endpoint`, and `--tikeo-api-token` flags.
- RBAC and sample manifest under `config/`.

`applyMode=diffOnly` is the default and only performs drift review. `applyMode=apply` is accepted as an operator intent, but bulk mutation remains delegated to typed tikeo CRUD APIs so RBAC, approval, and audit controls are not bypassed.

```bash
cd deploy/k8s/operator
go test ./...
go run ./cmd/tikeo-operator --tikeo-endpoint http://localhost:5173 --tikeo-api-token "$TIKEO_API_TOKEN"
```
