# tikee K8s deployment notes

This baseline deploys tikee server and web as separate containers. Workers are not exposed through Kubernetes Services; workers run in business namespaces or clusters and initiate outbound gRPC connections to `tikee-worker-tunnel.tikee.svc.cluster.local:9998`.

The included SQLite PVC is for development only. Production deployments should replace `[storage].database_url` with MySQL, PostgreSQL, CockroachDB, or another managed database endpoint and remove the single-writer SQLite PVC constraint.
