# Tikeo Helm chart ⛵

[🇨🇳 中文部署文档](../../../README.zh-CN.md#运行-tikeo-服务)

This chart installs the Tikeo Server management API, the Worker Tunnel endpoint, and the Web console. It intentionally does **not** deploy business workers or expose worker inbound ports: workers connect outbound to the Worker Tunnel service.

## Quick install

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo --create-namespace \
  -f deploy/helm/tikeo/examples/values-sqlite-dev.yaml
```

For release installs, pin image tags:

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo --create-namespace \
  --set server.image.repository=yhyzgn/tikeo-server \
  --set web.image.repository=yhyzgn/tikeo-web \
  --set server.image.tag=v0.1.0 \
  --set web.image.tag=v0.1.0
```

## Production database

Use PostgreSQL, MySQL, or CockroachDB through a Kubernetes Secret. Do not commit production database URLs to `values.yaml`.

```bash
kubectl -n tikeo create secret generic tikeo-database \
  --from-literal=database-url='postgres://tikeo:change-me@postgres.example:5432/tikeo?sslmode=require'

helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo --create-namespace \
  -f deploy/helm/tikeo/examples/values-external-postgres.yaml
```

Relevant values:

```yaml
server:
  storage:
    mode: external
    existingSecret: tikeo-database
    databaseUrlSecretKey: database-url
    persistence:
      enabled: false
```

The chart injects the secret as `TIKEO__STORAGE__DATABASE_URL`, which overrides the generated `container.toml` fallback through Tikeo's environment configuration loader.

## TLS and mTLS

Tikeo supports real HTTP TLS and Worker Tunnel TLS/mTLS listeners. The chart wires mounted Kubernetes Secrets into `[transport_security.http]` and `[transport_security.worker_tunnel]` in the generated config.

Expected Secret keys:

| Secret purpose | Required keys |
| --- | --- |
| HTTP listener TLS | `tls.crt`, `tls.key` |
| Worker Tunnel TLS | `tls.crt`, `tls.key` |
| Worker Tunnel client CA for mTLS | `ca.crt` |
| Optional HTTP client CA | `ca.crt` |

Example:

```bash
kubectl -n tikeo create secret tls tikeo-http-tls \
  --cert=./certs/http.crt --key=./certs/http.key
kubectl -n tikeo create secret tls tikeo-worker-tunnel-tls \
  --cert=./certs/worker-tunnel.crt --key=./certs/worker-tunnel.key
kubectl -n tikeo create secret generic tikeo-worker-client-ca \
  --from-file=ca.crt=./certs/worker-client-ca.crt

helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo --create-namespace \
  -f deploy/helm/tikeo/examples/values-external-postgres.yaml \
  -f deploy/helm/tikeo/examples/values-ingress-tls.yaml
```

Ingress TLS and listener TLS are separate boundaries: ingress TLS terminates traffic at the ingress controller, while `server.tls.http` and `server.tls.workerTunnel` configure the Tikeo process listeners themselves.

## Worker identity and networking

Business workers remain external to this chart. Deploy them as sidecars, Deployments, DaemonSets, VM/systemd services, or embedded SDK clients, then point them at the Tikeo Worker Tunnel endpoint.

Rules:

- Do not create inbound Services for business workers.
- Store bootstrap tokens and identity fields in Secrets or environment-specific deployment tooling.
- Use structured identity fields such as namespace, app, pool, cluster, region, labels, and structured capabilities instead of name matching.
- See `deploy/worker/identity.env.example` and `deploy/helm/tikeo/examples/values-worker-identity.yaml` for the worker identity shape.

## Probes, resources, and security contexts

The chart exposes tunable readiness/liveness probes, resources, pod annotations, node placement, and security contexts for both server and web workloads. Defaults keep local installs compatible with the published images; production overlays should set resource requests/limits and image-compatible security contexts explicitly.

## Rollback

Keep each production change in a separate Helm revision and verify `/readyz` plus at least one worker dry-run before declaring the rollout healthy.

```bash
helm history tikeo --namespace tikeo
helm status tikeo --namespace tikeo
kubectl -n tikeo rollout status deploy/tikeo-server
kubectl -n tikeo rollout status deploy/tikeo-web

# Roll back to a known-good revision.
helm rollback tikeo <REVISION> --namespace tikeo
kubectl -n tikeo rollout status deploy/tikeo-server
kubectl -n tikeo rollout status deploy/tikeo-web
```

If rollback involves database migrations, restore from the database provider's backup/snapshot process first; Helm rollback only reverts Kubernetes manifests and image/config revisions.

## Included examples

| File | Purpose |
| --- | --- |
| `values-sqlite-dev.yaml` | Single-node/dev install with SQLite PVC. |
| `values-external-postgres.yaml` | Production baseline using an external database secret. |
| `values-ingress-tls.yaml` | Ingress TLS plus Tikeo listener TLS/mTLS secret wiring. |
| `values-worker-identity.yaml` | Documentation-only shape for remote worker identity bootstrap. |
