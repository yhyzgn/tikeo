---
title: Kubernetes and Helm
description: Copy-paste Helm install, external database, TLS/mTLS, NetworkPolicy, ServiceMonitor, Gateway API, and rollback guide.
---

# Kubernetes and Helm

Use Helm when you need Kubernetes-native rollout history, Secrets, Services, Ingress, TLS/mTLS mounts, probes, resources, NetworkPolicy, and Prometheus Operator integration. The chart installs the Tikeo Server management API, Worker Tunnel endpoint, and Web console. It intentionally does **not** deploy business Workers or create Worker inbound Services.

## Prerequisites

```bash
kubectl version --client
helm version
kubectl create namespace tikeo --dry-run=client -o yaml | kubectl apply -f -
```

For production, prepare a database Secret and TLS/mTLS Secrets before install. For a local single-node evaluation, use the SQLite values file.

## 1. One-command dev install with SQLite PVC

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo --create-namespace \
  -f deploy/helm/tikeo/examples/values-sqlite-dev.yaml
kubectl -n tikeo rollout status deploy/tikeo-server
kubectl -n tikeo rollout status deploy/tikeo-web
kubectl -n tikeo port-forward svc/tikeo-server 9090:9090 >/tmp/tikeo-api.port-forward.log 2>&1 &
curl -fsS http://127.0.0.1:9090/readyz
```

Web port-forward:

```bash
kubectl -n tikeo port-forward svc/tikeo-web 8080:80 >/tmp/tikeo-web.port-forward.log 2>&1 &
open http://127.0.0.1:8080 || true
```

## 2. Production-shaped install with external database

Create the Secret first. PostgreSQL example:

```bash
kubectl -n tikeo create secret generic tikeo-database \
  --from-literal=database-url='postgres://tikeo:change-me@postgres.example:5432/tikeo?sslmode=require'
```

Install with the external DB overlay:

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo --create-namespace \
  -f deploy/helm/tikeo/examples/values-external-postgres.yaml \
  --set server.image.repository=yhyzgn/tikeo-server \
  --set web.image.repository=yhyzgn/tikeo-web \
  --set server.image.tag=dev \
  --set web.image.tag=dev
kubectl -n tikeo rollout status deploy/tikeo-server
kubectl -n tikeo rollout status deploy/tikeo-web
curl -fsS http://127.0.0.1:9090/readyz || true
```

The chart injects the database URL as `TIKEO__STORAGE__DATABASE_URL`, overriding generated config.

## 3. TLS and mTLS install

Create listener and client CA Secrets. Replace file paths with your own certificates:

```bash
kubectl -n tikeo create secret tls tikeo-http-tls \
  --cert=./certs/http.crt --key=./certs/http.key
kubectl -n tikeo create secret tls tikeo-worker-tunnel-tls \
  --cert=./certs/worker-tunnel.crt --key=./certs/worker-tunnel.key
kubectl -n tikeo create secret generic tikeo-worker-client-ca \
  --from-file=ca.crt=./certs/worker-client-ca.crt
```

Install:

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo --create-namespace \
  -f deploy/helm/tikeo/examples/values-external-postgres.yaml \
  -f deploy/helm/tikeo/examples/values-ingress-tls.yaml
```

Ingress TLS and Tikeo listener TLS are separate. Ingress TLS terminates traffic at the ingress controller; `server.tls.http` and `server.tls.workerTunnel` configure the Tikeo process listeners.

## 4. Optional operations hardening

Enable PDB, NetworkPolicy, and ServiceMonitor:

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo --create-namespace \
  -f deploy/helm/tikeo/examples/values-external-postgres.yaml \
  -f deploy/helm/tikeo/examples/values-ops-hardening.yaml
```

Render Gateway API Worker Tunnel route before applying it to a cluster with matching CRDs/controller:

```bash
helm template tikeo ./deploy/helm/tikeo \
  --namespace tikeo \
  -f deploy/helm/tikeo/examples/values-gateway-api-worker-tunnel.yaml
```

## Helm values reference

| Value | Default | Purpose |
|---|---:|---|
| `server.replicas` | `1` | Server pod replicas. Use external DB before scaling shared installs. |
| `server.httpPort` | `9090` | Container HTTP listener for API/health. |
| `server.workerTunnelPort` | `9998` | Container Worker Tunnel gRPC/HTTP2 listener. |
| `server.storage.mode` | `sqlite` | `sqlite` creates/uses PVC; `external` reads DB URL from Secret. |
| `server.storage.existingSecret` | empty | Secret containing database URL for external mode. |
| `server.storage.databaseUrlSecretKey` | `database-url` | Secret key read into `TIKEO__STORAGE__DATABASE_URL`. |
| `server.storage.persistence.enabled` | `true` | SQLite PVC toggle. Disable for external DB mode. |
| `server.tls.http.enabled` | `false` | Enable Tikeo HTTP listener TLS. |
| `server.tls.workerTunnel.enabled` | `false` | Enable Worker Tunnel listener TLS. |
| `server.tls.workerTunnel.mtlsRequired` | `false` | Require Worker client certificates. |
| `server.ingress.enabled` | `false` | Render API ingress. |
| `web.ingress.enabled` | `false` | Render Web ingress. |
| `networkPolicy.enabled` | `false` | Render NetworkPolicy while preserving outbound-only Worker model. |
| `serviceMonitor.enabled` | `false` | Render Prometheus Operator `ServiceMonitor`. |
| `gatewayApi.enabled` | `false` | Render Gateway API resources for Worker Tunnel. |

## Worker rule

Business Workers remain outside this chart. Deploy them as separate Deployments, sidecars, DaemonSets, VM/systemd services, or embedded SDK clients. They dial the Worker Tunnel Service or Gateway endpoint outbound. Do not create inbound business Worker Services.

## Validate and debug

```bash
helm lint deploy/helm/tikeo
helm template tikeo deploy/helm/tikeo --namespace tikeo
kubectl -n tikeo get pods,svc,ingress
kubectl -n tikeo logs deploy/tikeo-server --tail=120
kubectl -n tikeo describe pod -l app.kubernetes.io/component=server
```

## Rollback

```bash
helm history tikeo --namespace tikeo
helm status tikeo --namespace tikeo
helm rollback tikeo <REVISION> --namespace tikeo
kubectl -n tikeo rollout status deploy/tikeo-server
kubectl -n tikeo rollout status deploy/tikeo-web
```

Helm rollback reverts Kubernetes manifests and image/config revisions. It does not reverse database migrations; take a database snapshot before upgrades in shared environments.
