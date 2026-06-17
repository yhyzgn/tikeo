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


## 3. Server Raft HA 安装

应用 overlay 之前，先阅读 [Server 高可用与集群模式](./server-ha)，其中包含部署架构图、优缺点、模式选择和 Worker Tunnel failover 语义。

当 Server 控制面需要多个 Kubernetes Pod 时使用 Raft HA。该路径要求外部 PostgreSQL/MySQL/CockroachDB 数据库和 Raft transport Secret。Chart 会把 Server 从 `Deployment` 切换为 `StatefulSet`，创建 `tikeo-server-headless` peer Service，把每个 Pod 名称注入为 `TIKEO__CLUSTER__NODE_ID`，并渲染 `http://tikeo-server-0.tikeo-server-headless:9090` 这类静态 peer endpoint。

创建内部 transport token Secret：

```bash
kubectl -n tikeo create secret generic tikeo-raft-transport   --from-literal=transport-token="$(openssl rand -hex 32)"
```

安装：

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo   --namespace tikeo --create-namespace   -f deploy/helm/tikeo/examples/values-external-postgres.yaml   -f deploy/helm/tikeo/examples/values-raft-ha.yaml
kubectl -n tikeo rollout status statefulset/tikeo-server
kubectl -n tikeo get pods -l app.kubernetes.io/component=server
```

调度语义：所有 Server Pod 参与 Raft。只有一个已选出的 Leader 在持久化 fencing token 后报告 `canSchedule=true`；Leader 运行全局 timer/retry 所有权循环，并投影均衡的 shard ownership。派发按 shard 多 owner 执行：任一持有 active `cluster_shard_ownership` 行的 Pod 都只能 claim 并派发自己拥有的 queue shard；非 owner 和旧 fencing token 会 fail closed。所有 Pod 都可以继续承载 health/API/Raft transport 和 Worker Tunnel gateway 流量。Tikeo 核心调度所有权不使用 Redis/Dragonfly 分布式锁。

## 4. TLS/mTLS

```bash
kubectl -n tikeo create secret tls tikeo-http-tls --cert=./certs/http.crt --key=./certs/http.key
kubectl -n tikeo create secret tls tikeo-worker-tunnel-tls --cert=./certs/worker-tunnel.crt --key=./certs/worker-tunnel.key
kubectl -n tikeo create secret generic tikeo-worker-client-ca --from-file=ca.crt=./certs/worker-client-ca.crt

helm upgrade --install tikeo ./deploy/helm/tikeo   --namespace tikeo --create-namespace   -f deploy/helm/tikeo/examples/values-external-postgres.yaml   -f deploy/helm/tikeo/examples/values-ingress-tls.yaml
```

Ingress TLS 与 Tikeo listener TLS 是两个边界；Worker Tunnel mTLS 用于 Worker 客户端证书校验。

## 5. 运维增强

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
| `server.replicas` | `1` | Server 副本数；standalone 保持 1，多 Pod Server HA 使用 `server.cluster.mode=raft` 和外部 DB。 |
| `server.httpPort` | `9090` | API/health 容器端口。 |
| `server.workerTunnelPort` | `9998` | Worker Tunnel 容器端口。 |
| `server.cluster.mode` | `standalone` | `standalone` 或 `raft`；raft 渲染 StatefulSet/headless peer 拓扑。Leader 负责 fencing/projection，active shard owner 派发自己的 shard。 |
| `server.cluster.transportTokenExistingSecret` | 空 | raft 模式必填，保存内部 transport token 的 Secret。 |
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

## 前置条件

执行本页命令前，请先满足页面列出的安装、认证和权限要求。本地示例默认 Server 使用 `config/dev.toml`，客户端访问 `127.0.0.1`，令牌保存在 shell 变量中，不写入文件或截图。

## 验收

完成本页步骤后，用对应 API、UI、构建、smoke 或部署检查验证结果。有效验收至少包含执行的命令、检查的路由或文件，以及观察到的状态或产物。

## 故障排查

步骤失败时，先保留完整命令、响应状态和 Server 日志时间窗口，再检查认证、namespace/app scope、Worker 匹配、存储 readiness 和代理行为，不要直接修改生产配置。

## 生产检查清单

- [ ] 密钥通过环境变量或平台 Secret 引用管理，不写入示例。
- [ ] 已把本地 `127.0.0.1` 命令替换成真实域名、TLS 和认证方式。
- [ ] 已记录变更面的回滚和证据采集方式。
- [ ] 运维人员可以在没有隐藏 shell 历史或隐式状态的情况下复现验收。
