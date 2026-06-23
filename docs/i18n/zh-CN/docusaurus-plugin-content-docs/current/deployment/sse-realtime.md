---
title: SSE 实时刷新部署注意事项
description: Tikeo 控制台实时刷新在代理、负载均衡、WAF 和 Kubernetes Ingress 后的配置手册。
---

# SSE 实时刷新部署注意事项

Tikeo Web 使用 Server-Sent Events（SSE）刷新控制台中的实时状态。SSE 是长连接 HTTP `GET` 响应，`Content-Type` 为 `text/event-stream`；它不是 WebSocket，也不应该被当成短 JSON 响应缓存或缓冲。

## 当前 stream 端点

| UI 区域 | Endpoint pattern | 流行为 |
|---|---|---|
| Workflow 实例时间线 | `/api/v1/events/instances/{id}/stream` | 回放 workflow events，并持续发送新的时间线事件。 |
| Job 实例日志抽屉 | `/api/v1/instances/{id}/logs/stream` | 发送实例快照和增量 `instance.log` 事件。 |
| Dashboard 与实例列表 | `/api/v1/instances/stream` | 推送 Jobs/instances 快照，用于实例趋势与状态面板。 |
| Dashboard 与 Worker 集群页 | `/api/v1/workers/stream` | Worker/lifecycle 快照变化时推送。 |
| Dashboard 与调度队列页 | `/api/v1/dispatch-queue/stream` | 队列快照变化时推送。 |

浏览器 `EventSource` 不能设置 `Authorization` header，因此 Tikeo Web 会使用 `?token=...` fallback。共享环境必须使用 HTTPS，并在 nginx、LB、WAF、Ingress 和 access log 中脱敏或过滤 `token` query 参数。

## 前置条件

先确认普通 API 正常，再排查 SSE：

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
```

准备一个本地 token：

```bash
export TIKEO_TOKEN='<local-session-token>'
```

直连 Server 的 smoke：

```bash
curl -N \
  -H 'Accept: text/event-stream' \
  "http://127.0.0.1:9090/api/v1/workers/stream?token=${TIKEO_TOKEN}"
```

验收标准：请求保持打开，响应 header 包含 `Content-Type: text/event-stream`，连接不会被代理固定在 30-60 秒关闭。Dashboard 还会每 3 秒用 REST 兜底刷新 cluster diagnostics、通知投递队列状态、审计日志和 Job instance 历史；SSE 问题通常表现为实时面板滞后，而 REST/代理/认证问题可能导致整个驾驶舱为空。

## 网络层要求

所有代理、负载均衡、WAF、Ingress 和 CDN hop 都要满足：

- 允许长时间打开、没有固定 `Content-Length` 的 `GET` 响应；
- 立即转发响应 chunk，不等待完整 body；
- 对 stream 路径关闭 response buffering；
- 对 `text/event-stream` 关闭 gzip/compression 缓冲；
- `/api/v1/**/stream` 不走缓存；
- idle/read timeout 高于应用 keep-alive 周期；
- 为多控制台 tab 预留足够客户端/同源连接数；
- access log 不记录原始 `token` query 值。

Tikeo 的 SSE keep-alive 间隔是 15 秒。所有 hop 的 idle timeout 都应明显高于 15 秒；`60s` 是实用下限，运维控制台建议 `300s` 到 `3600s`。

## nginx 反向代理

可以把以下配置放到 Tikeo API 的 location；如果静态 Web 与 API 分开，也可以只作用于 `/api/v1/` 或 stream location。

```nginx
location /api/v1/ {
    proxy_pass http://tikeo-server:9090;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;

    # SSE：不要缓冲或缓存流式响应。
    proxy_buffering off;
    proxy_cache off;
    gzip off;

    # 保持 upstream stream 打开，并确保高于所有 LB/WAF hop 的 timeout。
    proxy_read_timeout 3600s;
    proxy_send_timeout 3600s;

    # 避免 HTTP/1.1 upstream 的 hop-by-hop Connection header 问题。
    proxy_set_header Connection "";
}
```

常见 nginx 现象：

| 现象 | 可能原因 | 处理 |
|---|---|---|
| 事件攒成一批才到达 | `proxy_buffering` 或 gzip 缓冲 | 对 SSE location 关闭 buffering/compression。 |
| 浏览器每 30-60 秒重连 | `proxy_read_timeout` 或上游 LB idle timeout | 提高所有 idle/read timeout。 |
| live view 出现 `499`、`502`、`504` | 代理关闭长响应或 timeout 不一致 | 对齐 nginx、LB、Ingress timeout，并检查 upstream 健康。 |
| access log 里出现 token | `EventSource` token query fallback | 在日志中脱敏 `token` query，或使用同源部署并限制日志访问。 |

## 负载均衡与 CDN

L4/L7 负载均衡需要把 SSE 路径当作普通 HTTP streaming 转发。不要让它经过为静态资源、短响应或聚合响应优化的缓存层。

建议配置：

- idle timeout：至少 `60s`，控制台场景建议 `300s+`；
- response buffering：关闭；
- upstream 协议：HTTP/1.1 最少意外；HTTP/2 可以使用，但必须确认代理会立即 flush streaming frame；
- health check：使用 `/readyz` 或 `/healthz`，不要使用 stream endpoint；
- stickiness：正确性不依赖粘性，但应避免用户看实时流时频繁 drain backend；
- CDN cache：绕过 `/api/v1/`，尤其是 `/api/v1/**/stream`。

云产品提示：

- AWS ALB：将 `idle_timeout.timeout_seconds` 设置为 `300` 或 `3600`。
- Cloudflare 或 WAF/CDN：确认套餐与规则允许长连接 streaming HTTP；否则对 `/api/v1/**/stream` 旁路代理。
- Envoy、HAProxy、Traefik：关闭 response buffering，提高 stream/idle timeout。

## WAF 规则

SSE endpoint 对通用 WAF 比较特殊：它是带认证的长连接 `GET`，持续输出事件帧。需要确认 WAF 不会：

- 要求响应必须有 `Content-Length`；
- 阻止 `text/event-stream`；
- 把长时间 `GET` 响应误判为 slowloris 或数据外传；
- 删除或改写 `token` query 参数；
- 等响应 body 达到某个最小大小才放行；
- 向 API 响应注入 JS challenge 页面。

安全要求：

- 优先使用 Web/API 同源部署；
- 只在 HTTPS 下跨网络暴露 token query fallback；
- 在代理、WAF、LB 日志中脱敏 query string，至少脱敏 `token`；
- stream endpoint 的权限与对应 REST endpoint 对齐；
- 不把 SSE URL 当作可分享链接。

## Kubernetes Ingress

### ingress-nginx

服务 Tikeo API 的 Ingress 可使用类似注解：

```yaml
metadata:
  annotations:
    nginx.ingress.kubernetes.io/proxy-buffering: "off"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-request-buffering: "off"
```

如果集群禁用了 annotation snippet，就在 IngressClass 或 controller ConfigMap 层面配置。避免 rewrite rule 把 `/api/v1/**/stream` 改写到非 Tikeo Server route。

### AWS ALB Ingress Controller

```yaml
metadata:
  annotations:
    alb.ingress.kubernetes.io/load-balancer-attributes: idle_timeout.timeout_seconds=3600
```

同时确认 target group health check 使用 `/readyz`，并且绑定到 ALB 的 WAF 允许 streaming `GET` 响应。

### 其他 controller

- Traefik：提高 forwarding/response timeout，不要对 `/api/v1/` 启用 buffering middleware。
- Envoy/Gateway API：提高 route/stream idle timeout，并关闭会缓冲完整响应的 filter。
- NGINX Inc. controller：根据 controller 版本使用等价的 `nginx.org/proxy-read-timeout`、`nginx.org/proxy-send-timeout` 和 buffering 控制项。

## 验收

部署后至少验证三条路径；下面用本机端口转发或本地 TLS 入口举例，实际域名按环境替换：

```bash
curl -N \
  -H 'Accept: text/event-stream' \
  "https://127.0.0.1:9443/api/v1/workers/stream?token=${TIKEO_TOKEN}"

curl -N \
  -H 'Accept: text/event-stream' \
  "https://127.0.0.1:9443/api/v1/dispatch-queue/stream?token=${TIKEO_TOKEN}"

curl -N \
  -H 'Accept: text/event-stream' \
  "https://127.0.0.1:9443/api/v1/instances/${TIKEO_INSTANCE_ID}/logs/stream?token=${TIKEO_TOKEN}"
```

验收标准：

- 请求保持 pending/open；
- header 是 `text/event-stream`；
- 初始快照或 keep-alive 能持续到达；
- 代理日志没有原始 token；
- 浏览器 DevTools 没有固定间隔重连；
- 触发 Worker、实例或 dispatch queue 变化后，Dashboard 的实例趋势、Worker Mesh/能力覆盖和队列压力面板无需完整刷新即可更新。

## 排障

1. 打开浏览器 DevTools，确认请求保持 pending，且 `Content-Type` 是 `text/event-stream`。
2. 确认响应不是静态 CDN、登录页、WAF challenge 或缓存对象。
3. 观察重连频率；固定间隔重连通常是 idle timeout。
4. 对比直连 Server 与代理访问：

   ```bash
   curl -N \
     -H 'Accept: text/event-stream' \
     "http://127.0.0.1:9090/api/v1/workers/stream?token=${TIKEO_TOKEN}"
   ```

5. 检查代理/WAF 日志中的 `499`、`502`、`504`、`524`、`403` 或 challenge 页面。
6. 确认 query token 被保留，没有被 Ingress rewrite 或 WAF rule 删除。
7. 如果 Web 与 API 不同源，确认前端 API origin/CORS 策略。
8. 如果只有某个页面不刷新，确认对应 stream endpoint 是否被单独 rewrite 或缓存。

## 清理/生产检查清单

- 探针和监控使用 `/readyz` 或 `/healthz`，不要用 SSE endpoint 做 probe。
- `/api/v1/**/stream` 关闭缓存、缓冲和 gzip 缓冲。
- 所有 hop 的 idle/read timeout 高于 15 秒 keep-alive，生产建议至少 `300s`。
- 代理和 WAF 日志脱敏 `token` query。
- HTTPS 下暴露跨网络 SSE；同源部署优先。
- CDN 对 API 和 stream 路径旁路缓存。
- 发布前用浏览器和 `curl -N` 同时验证。

## 故障排查

步骤失败时，先保留完整命令、响应状态和 Server 日志时间窗口，再检查认证、namespace/app scope、Worker 匹配、存储 readiness 和代理行为，不要直接修改生产配置。

## 生产检查清单

- [ ] 密钥通过环境变量或平台 Secret 引用管理，不写入示例。
- [ ] 已把本地 `127.0.0.1` 命令替换成真实域名、TLS 和认证方式。
- [ ] 已记录变更面的回滚和证据采集方式。
- [ ] 运维人员可以在没有隐藏 shell 历史或隐式状态的情况下复现验收。
