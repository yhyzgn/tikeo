---
title: SSE realtime deployment notes
description: Proxy, load balancer, WAF, and Kubernetes Ingress requirements for Tikeo Server-Sent Events.
---

# SSE realtime deployment notes

Tikeo Web uses Server-Sent Events (SSE) for realtime console updates. The current streaming HTTP endpoints are:

| UI area | Endpoint pattern | Stream behavior |
|---|---|---|
| Workflow instance timeline | `/api/v1/events/instances/{id}/stream` | Replays workflow events and keeps sending new timeline events. |
| Job instance log drawer | `/api/v1/instances/{id}/logs/stream` | Sends instance snapshots and incremental `instance.log` events. |
| Worker cluster page | `/api/v1/workers/stream` | Sends changed worker/lifecycle snapshots. |
| Dispatch queue page | `/api/v1/dispatch-queue/stream` | Sends changed queue snapshots. |

Browsers use `EventSource`, which cannot attach an `Authorization` header. Tikeo therefore supports the same `?token=...` fallback used by the Web console. Always serve Web and API over HTTPS in shared environments and redact the `token` query parameter from access logs.

## Network requirements

SSE is a long-lived HTTP response with `Content-Type: text/event-stream`. It is not WebSocket, but it is still sensitive to proxy defaults that assume short JSON responses.

Production network layers must allow:

- long-lived `GET` responses without a fixed `Content-Length`;
- response streaming without proxy buffering or CDN/WAF buffering;
- idle/read timeouts longer than the application keep-alive cadence;
- no gzip/compression buffering for `text/event-stream`;
- no caching for `/api/v1/**/stream` responses;
- enough per-client and per-origin connection capacity for multiple open console tabs.

Tikeo emits SSE keep-alives every 15 seconds. Configure every proxy/LB/WAF hop with an idle timeout comfortably above that value; `60s` is a practical minimum and `300s` to `3600s` is safer for operator consoles.

## nginx reverse proxy

Apply these settings to the API location that forwards Tikeo HTTP traffic, or to a narrower `/api/v1/` / `*stream` location if your nginx routing separates API and Web assets.

```nginx
location /api/v1/ {
    proxy_pass http://tikeo-server:9090;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;

    # Required for SSE: do not buffer or cache streaming responses.
    proxy_buffering off;
    proxy_cache off;
    gzip off;

    # Keep the upstream stream open. Use values higher than every LB/WAF hop.
    proxy_read_timeout 3600s;
    proxy_send_timeout 3600s;

    # Avoid hop-by-hop connection header surprises when proxying to upstream HTTP/1.1.
    proxy_set_header Connection "";
}
```

Common nginx symptoms:

| Symptom | Likely cause | Fix |
|---|---|---|
| Events arrive in large batches instead of live | `proxy_buffering` or gzip buffering | Disable buffering/compression for SSE locations. |
| Browser reconnects every 30-60 seconds | `proxy_read_timeout` or upstream LB idle timeout | Raise all idle/read timeouts above the keep-alive interval. |
| `499`, `502`, `504` during live views | Proxy closes long responses or upstream timeout mismatch | Align nginx, LB, and ingress timeouts; check upstream pod/server health separately. |
| Token appears in access logs | EventSource token query fallback | Redact `token` query parameters or use a same-origin deployment with controlled logs. |

## Load balancers and CDNs

For L4/L7 load balancers, keep the SSE path as plain HTTP streaming. Do not route it through products configured for static response caching or response aggregation.

Recommended settings:

- idle timeout: at least `60s`; prefer `300s+` for console sessions;
- response buffering: disabled;
- HTTP protocol: HTTP/1.1 upstream is the least surprising path; HTTP/2 is acceptable only if the proxy preserves streaming flushes;
- health checks: use `/readyz` or `/healthz`, never a stream endpoint;
- stickiness: not required for correctness, but avoid draining a backend while operators are watching active streams;
- CDN cache: bypass `/api/v1/` and especially `/api/v1/**/stream`.

Cloud-specific examples:

- AWS ALB: set `idle_timeout.timeout_seconds` to a value such as `300` or `3600`.
- Cloudflare or WAF/CDN products: ensure the plan and rule set allow long-lived streaming HTTP responses; if not, bypass proxying for `/api/v1/**/stream`.
- Envoy/HAProxy/Traefik: disable response buffering and raise stream/idle timeouts for the Tikeo API route.

## WAF rules

SSE endpoints look unusual to generic WAF profiles because they are long-lived authenticated `GET` requests with streaming JSON event frames. Make sure WAF rules do not:

- require `Content-Length` on responses;
- block `text/event-stream`;
- classify long-lived `GET` responses as slowloris or data exfiltration;
- strip or rewrite the `token` query parameter;
- buffer the response until a minimum body size is reached;
- inject JavaScript/challenge pages into API responses.

Security guidance:

- prefer same-origin Web/API deployments where possible;
- use HTTPS before exposing token query fallback across a network;
- redact query strings or at least `token` in proxy, WAF, and LB logs;
- keep stream endpoint permissions aligned with the matching REST endpoint.

## Kubernetes Ingress

### ingress-nginx

Use annotations similar to the following on the Ingress serving the Tikeo API:

```yaml
metadata:
  annotations:
    nginx.ingress.kubernetes.io/proxy-buffering: "off"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-request-buffering: "off"
```

If your cluster disables annotation snippets, keep the configuration at the IngressClass/controller ConfigMap level instead of relying on per-Ingress snippets. Avoid rewrite rules that move `/api/v1/**/stream` away from the Tikeo server route.

### AWS ALB Ingress Controller

```yaml
metadata:
  annotations:
    alb.ingress.kubernetes.io/load-balancer-attributes: idle_timeout.timeout_seconds=3600
```

Also make sure target group health checks use `/readyz` and that any WAF attached to the ALB allows streaming `GET` responses.

### Other controllers

- Traefik: set forwarding/response timeouts high enough and do not enable buffering middleware for `/api/v1/`.
- Envoy/Gateway API: raise route/stream idle timeouts and disable filters that buffer full responses.
- NGINX Inc. controller: use the equivalent `nginx.org/proxy-read-timeout`, `nginx.org/proxy-send-timeout`, and buffering controls for your controller version.

## Troubleshooting checklist

1. Open browser DevTools and confirm the request stays pending with `Content-Type: text/event-stream`.
2. Check that the response is not cached and is not served by a static CDN layer.
3. Watch reconnect frequency. Reconnects at a fixed interval usually mean an idle timeout.
4. Compare direct server access (`curl -N http://server:9090/.../stream`) with proxied access.
5. Inspect proxy/WAF logs for `499`, `502`, `504`, `524`, `403`, or challenge-page responses.
6. Verify that query tokens are accepted and not stripped by an ingress rewrite rule.
7. Confirm frontend and API origin/CORS policy if Web is not served from the same origin as the API.

Example smoke command:

```bash
curl -N \
  -H 'Accept: text/event-stream' \
  "https://tikeo.example.com/api/v1/workers/stream?token=${TIKEO_TOKEN}"
```

Use `/readyz` for liveness/readiness probes and monitoring. Do not use SSE endpoints for probes; each probe would open a long-lived stream by design.
