---
title: SSE 实时刷新部署注意事项
description: Tikeo Server-Sent Events 在代理、负载均衡、WAF 和 Kubernetes Ingress 后面的配置要求。
---

# SSE 实时刷新部署注意事项

Tikeo Web 使用 Server-Sent Events（SSE）刷新控制台中的实时状态。当前流式 HTTP endpoint 包括：

| UI 区域 | Endpoint pattern | 流行为 |
|---|---|---|
| Workflow 实例时间线 | `/api/v1/events/instances/{id}/stream` | 回放 workflow events，并持续发送新的时间线事件。 |
| Job 实例日志抽屉 | `/api/v1/instances/{id}/logs/stream` | 发送实例快照和增量 `instance.log` 事件。 |
| Worker 集群页 | `/api/v1/workers/stream` | Worker/lifecycle 快照变化时推送。 |
| 调度队列页 | `/api/v1/dispatch-queue/stream` | 队列快照变化时推送。 |

浏览器 `EventSource` 不能设置 `Authorization` header，因此 Tikeo Web 使用 `?token=...` fallback。共享环境中必须使用 HTTPS，并在 nginx/LB/WAF/access log 中脱敏或过滤 `token` query 参数。

## 网络层要求

SSE 是一个长连接 HTTP 响应，`Content-Type` 为 `text/event-stream`。它不是 WebSocket，但会受到“短 JSON 响应”默认代理配置的影响。

生产网络层必须允许：

- 长时间打开、没有固定 `Content-Length` 的 `GET` 响应；
- 响应流式转发，不能被代理/CDN/WAF 缓冲；
- idle/read timeout 高于应用 keep-alive 周期；
- 对 `text/event-stream` 禁用 gzip/compression 缓冲；
- `/api/v1/**/stream` 不走缓存；
- 为多控制台 tab 预留足够的客户端/同源连接数。

Tikeo 的 SSE keep-alive 间隔是 15 秒。所有代理、LB、WAF、Ingress hop 的 idle timeout 都应该明显高于该值；`60s` 是实用下限，面向运维控制台建议 `300s` 到 `3600s`。

## nginx 反向代理

可以把以下配置放到转发 Tikeo API 的 location；如果你的 nginx 将静态 Web 与 API 分开，也可以只作用于 `/api/v1/` 或 `*stream` location。

```nginx
location /api/v1/ {
    proxy_pass http://tikeo-server:9090;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;

    # SSE 必需：不要缓冲或缓存流式响应。
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

常见 nginx 症状：

| 现象 | 可能原因 | 处理 |
|---|---|---|
| 事件攒成一批才到达 | `proxy_buffering` 或 gzip 缓冲 | 对 SSE location 关闭 buffering/compression。 |
| 浏览器每 30-60 秒重连 | `proxy_read_timeout` 或上游 LB idle timeout | 提高所有 idle/read timeout。 |
| live view 出现 `499`、`502`、`504` | 代理关闭长响应或 timeout 不一致 | 对齐 nginx、LB、Ingress timeout，并单独检查 upstream 健康。 |
| access log 里出现 token | `EventSource` token query fallback | 在日志中脱敏 `token` query，或使用同源部署并限制日志访问。 |

## 负载均衡与 CDN

L4/L7 负载均衡需要把 SSE 路径当作普通 HTTP streaming 转发。不要让它经过为静态资源、短响应或聚合响应优化的缓存层。

建议配置：

- idle timeout：至少 `60s`，控制台场景建议 `300s+`；
- response buffering：关闭；
- HTTP 协议：upstream HTTP/1.1 最少惊喜；HTTP/2 可以使用，但必须确认代理会立即 flush streaming frame；
- health check：使用 `/readyz` 或 `/healthz`，不要使用 stream endpoint；
- stickiness：正确性不依赖粘性，但避免在用户看实时流时频繁 drain backend；
- CDN cache：绕过 `/api/v1/`，尤其是 `/api/v1/**/stream`。

云产品示例：

- AWS ALB：将 `idle_timeout.timeout_seconds` 设置为 `300` 或 `3600` 这类值。
- Cloudflare 或 WAF/CDN：确认套餐与规则允许长连接 streaming HTTP；否则对 `/api/v1/**/stream` 旁路代理。
- Envoy/HAProxy/Traefik：关闭 response buffering，提高 stream/idle timeout。

## WAF 规则

SSE endpoint 对通用 WAF 来说比较特殊：它是带认证的长连接 `GET`，持续输出 JSON 事件帧。需要确认 WAF 不会：

- 要求响应必须有 `Content-Length`；
- 阻止 `text/event-stream`；
- 把长时间 `GET` 响应误判为 slowloris 或数据外传；
- 删除或改写 `token` query 参数；
- 等响应 body 达到某个最小大小才放行；
- 向 API 响应注入 JS challenge 页面。

安全建议：

- 优先使用 Web/API 同源部署；
- 只在 HTTPS 下跨网络暴露 token query fallback；
- 在代理、WAF、LB 日志中脱敏 query string 或至少脱敏 `token`；
- stream endpoint 的权限应与对应 REST endpoint 对齐。

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

如果集群禁用了 annotation snippet，就在 IngressClass 或 controller ConfigMap 层面配置。避免 rewrite rule 把 `/api/v1/**/stream` 改写到非 Tikeo server route。

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

## 排查清单

1. 打开浏览器 DevTools，确认请求保持 pending，且 `Content-Type` 是 `text/event-stream`。
2. 确认响应没有被缓存，也不是由静态 CDN 层返回。
3. 观察重连频率；固定间隔重连通常是 idle timeout。
4. 对比直连 server 与代理访问：`curl -N http://server:9090/.../stream`。
5. 检查代理/WAF 日志中的 `499`、`502`、`504`、`524`、`403` 或 challenge 页面。
6. 确认 query token 被保留，没有被 Ingress rewrite 或 WAF rule 删除。
7. 如果 Web 与 API 不同源，确认前端 API origin/CORS 策略。

Smoke 示例：

```bash
curl -N \
  -H 'Accept: text/event-stream' \
  "https://tikeo.example.com/api/v1/workers/stream?token=${TIKEO_TOKEN}"
```

探针和监控请使用 `/readyz` 或 `/healthz`。不要用 SSE endpoint 做 probe；每次 probe 都会按设计打开一个长连接。
