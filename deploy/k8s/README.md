# tikeo K8s deployment notes

This baseline deploys tikeo server and web as separate containers. Workers are not exposed through Kubernetes Services; workers run in business namespaces or clusters and initiate outbound gRPC connections to `tikeo-worker-tunnel.tikeo.svc.cluster.local:9998`.

The included SQLite PVC is for development only. Production deployments should replace `[storage].database_url` with MySQL, PostgreSQL, CockroachDB, or another managed database endpoint and remove the single-writer SQLite PVC constraint.

## CRD/operator

- `deploy/k8s/crd/tikeo-manifest-crd.yaml` defines the namespaced `TikeoManifest` CRD with a status subresource.
- `deploy/k8s/operator` contains the controller/operator implementation. It watches desired manifests, calls tikeo `/api/v1/gitops/diff`, and writes status evidence for drift review.
- `applyMode=diffOnly` is the default. `applyMode=apply` records an apply intent but does not bypass tikeo typed CRUD APIs, RBAC, approval, or audit controls.
