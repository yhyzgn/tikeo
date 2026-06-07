# Tikeo K8s CRD controller/operator 🤖

[🇨🇳 中文部署文档](../../../docs/zh-CN/deployment.md)

The operator watches `TikeoManifest` resources, calls the Tikeo GitOps diff API, and writes status
evidence for drift review.

```bash
cd deploy/k8s/operator
go test ./...
go run ./cmd/tikeo-operator --tikeo-endpoint http://localhost:9090 --tikeo-api-token "$TIKEO_API_TOKEN"
```

Operational cautions: do not treat operator apply mode as a bypass around Tikeo RBAC, audit, or typed
resource validation.
