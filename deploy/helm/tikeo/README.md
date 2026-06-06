# tikeo Helm chart

Development install example:

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo --namespace tikeo --create-namespace
```

For release installs, set the image tags to the release version:

```bash
helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo --create-namespace \
  --set server.image.tag=v0.1.0 \
  --set web.image.tag=v0.1.0
```
