# Tikeo Helm chart ⛵

[🇨🇳 中文部署文档](../../../docs/zh-CN/deployment.md)

Install the chart into a Kubernetes namespace:

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo --namespace tikeo --create-namespace
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

Operational cautions: use external databases, platform secrets, TLS ingress, and centralized log
collection for production clusters.
