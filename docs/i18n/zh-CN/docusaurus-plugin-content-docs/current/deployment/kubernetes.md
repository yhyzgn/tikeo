---
title: Kubernetes 与 Helm
description: Helm dev/prod/TLS/ops overlay、values 参数、Worker 规则与回滚命令。
---

# Kubernetes 与 Helm

Helm 适合需要 rollout history、Secrets、Services、Ingress、TLS/mTLS、probes、resources、NetworkPolicy 和 Prometheus Operator 的环境。chart 安装 Tikeo Server、Worker Tunnel endpoint 与 Web 控制台；不会部署业务 Worker，也不会创建业务 Worker 入站 Service。

## 前置检查

```bash
kubectl version --client
helm version
kubectl create namespace tikeo --dry-run=client -o yaml | kubectl apply -f -
```

## 1. SQLite PVC 开发安装

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo   --namespace tikeo --create-namespace   -f deploy/helm/tikeo/examples/values-sqlite-dev.yaml
kubectl -n tikeo rollout status deploy/tikeo-server
kubectl -n tikeo rollout status deploy/tikeo-web
kubectl -n tikeo port-forward svc/tikeo-server 9090:9090 >/tmp/tikeo-api.port-forward.log 2>&1 &
curl -fsS http://127.0.0.1:9090/readyz
```

Web：

```bash
kubectl -n tikeo port-forward svc/tikeo-web 8080:80 >/tmp/tikeo-web.port-forward.log 2>&1 &
```

## 2. 外部数据库生产形态

```bash
kubectl -n tikeo create secret generic tikeo-database   --from-literal=database-url='postgres://tikeo:change-me@postgres.example:5432/tikeo?sslmode=require'

helm upgrade --install tikeo ./deploy/helm/tikeo   --namespace tikeo --create-namespace   -f deploy/helm/tikeo/examples/values-external-postgres.yaml   --set server.image.repository=yhyzgn/tikeo-server   --set web.image.repository=yhyzgn/tikeo-web   --set server.image.tag=dev   --set web.image.tag=dev
```

chart 会把 Secret 注入为 `TIKEO__STORAGE__DATABASE_URL`。

## 3. TLS/mTLS

```bash
kubectl -n tikeo create secret tls tikeo-http-tls --cert=./certs/http.crt --key=./certs/http.key
kubectl -n tikeo create secret tls tikeo-worker-tunnel-tls --cert=./certs/worker-tunnel.crt --key=./certs/worker-tunnel.key
kubectl -n tikeo create secret generic tikeo-worker-client-ca --from-file=ca.crt=./certs/worker-client-ca.crt

helm upgrade --install tikeo ./deploy/helm/tikeo   --namespace tikeo --create-namespace   -f deploy/helm/tikeo/examples/values-external-postgres.yaml   -f deploy/helm/tikeo/examples/values-ingress-tls.yaml
```

Ingress TLS 与 Tikeo listener TLS 是两个边界；Worker Tunnel mTLS 用于 Worker 客户端证书校验。

## 4. 运维增强

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo   --namespace tikeo --create-namespace   -f deploy/helm/tikeo/examples/values-external-postgres.yaml   -f deploy/helm/tikeo/examples/values-ops-hardening.yaml
```

Gateway API 先渲染确认：

```bash
helm template tikeo ./deploy/helm/tikeo   --namespace tikeo   -f deploy/helm/tikeo/examples/values-gateway-api-worker-tunnel.yaml
```

## Helm values 参数表

| Value | 默认 | 说明 |
|---|---:|---|
| `server.replicas` | `1` | Server 副本数。共享环境先用外部 DB。 |
| `server.httpPort` | `9090` | API/health 容器端口。 |
| `server.workerTunnelPort` | `9998` | Worker Tunnel 容器端口。 |
| `server.storage.mode` | `sqlite` | `sqlite` 使用 PVC；`external` 从 Secret 读 DB URL。 |
| `server.storage.existingSecret` | 空 | 外部 DB Secret 名。 |
| `server.storage.databaseUrlSecretKey` | `database-url` | Secret key。 |
| `server.tls.http.enabled` | `false` | 启用 Tikeo HTTP listener TLS。 |
| `server.tls.workerTunnel.enabled` | `false` | 启用 Worker Tunnel TLS。 |
| `server.tls.workerTunnel.mtlsRequired` | `false` | 要求 Worker 客户端证书。 |
| `networkPolicy.enabled` | `false` | 渲染 NetworkPolicy，但不改变 Worker 主动出站模型。 |
| `serviceMonitor.enabled` | `false` | 渲染 Prometheus Operator ServiceMonitor。 |
| `gatewayApi.enabled` | `false` | 渲染 Gateway API Worker Tunnel 资源。 |

## Worker 规则

业务 Worker 不属于此 chart。它们应作为独立 Deployment、DaemonSet、sidecar、VM/systemd 服务或 SDK 进程主动连接 Worker Tunnel。不要创建业务 Worker 入站 Service。

## 验证与回滚

```bash
helm lint deploy/helm/tikeo
helm template tikeo deploy/helm/tikeo --namespace tikeo
kubectl -n tikeo get pods,svc,ingress
kubectl -n tikeo logs deploy/tikeo-server --tail=120
helm history tikeo --namespace tikeo
helm rollback tikeo <REVISION> --namespace tikeo
```

Helm rollback 只回滚 Kubernetes manifest/image/config；数据库 migration 需要数据库快照配合。

## 适用边界

Helm 路径适合生产形态验证，但仍需要集群自身提供 Ingress controller、证书管理、Secret 管理、NetworkPolicy 实现和可选 Prometheus Operator。文档中的命令可以直接复制执行；生产环境应把镜像 tag、数据库 Secret、证书 Secret 与资源配额替换为自己的值。

## 参数替换建议

开发安装可以直接使用 SQLite PVC overlay；生产安装应使用外部数据库 Secret，并固定镜像 tag。启用 mTLS 时，Worker 端必须持有受 `tikeo-worker-client-ca` 信任的客户端证书，否则注册会失败。这是预期的安全边界，不应该通过关闭校验来绕过。
