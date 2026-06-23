---
title: SSE realtime deployment notes
description: Operator manual for proxy, load balancer, WAF, and Kubernetes Ingress settings required by Tikeo Server-Sent Events.
---

# SSE realtime deployment notes

Tikeo Web uses Server-Sent Events (SSE) for realtime console updates. SSE is a long-lived HTTP `GET` response with `Content-Type: text/event-stream`. It is not WebSocket traffic, but it fails when proxies buffer responses, compress streams, cache API paths, or close idle requests too quickly.

## Streaming routes

| UI area | Route | Event examples | Permission |
| --- | --- | --- | --- |
| Workflow instance timeline | `/api/v1/events/instances/{id}/stream` | workflow event types from the instance event log | `workflows:read` |
| Job instance log drawer | `/api/v1/instances/{id}/logs/stream` | `instance.snapshot`, `instance.log` | `instances:read` |
| Dashboard and instance list | `/api/v1/instances/stream` | `instances.snapshot` | `instances:read` |
| Dashboard and Worker cluster page | `/api/v1/workers/stream` | `workers.snapshot` | `workers:read` |
| Dashboard and dispatch queue page | `/api/v1/dispatch-queue/stream` | `dispatchQueue.snapshot` | `workers:read` |

Browsers use `EventSource`, which cannot attach an `Authorization` header. Tikeo therefore supports a `?token=...` query fallback for stream routes. Use HTTPS in shared environments and redact query strings, or at least the `token` parameter, from proxy, WAF, load balancer, and application access logs.

## Prerequisites

Before changing proxies, prove the Server stream works directly:

```bash
curl -fsS http://127.0.0.1:9090/readyz
curl -N http://127.0.0.1:9090/api/v1/workers/stream \
  -H "authorization: Bearer $TOKEN" \
  -H 'Accept: text/event-stream'
```

Expected behavior:

- the response stays open;
- headers include `content-type: text/event-stream`;
- a `workers.snapshot` event appears when the visible worker snapshot changes;
- keep-alives are sent every 15 seconds.

If direct access fails, fix auth, Server readiness, or route permissions before debugging nginx, load balancers, or Ingress. The Dashboard also polls REST endpoints every 3 seconds for cluster diagnostics, alert delivery queue status, audit logs, and job instance history; SSE problems usually make realtime panels stale, while REST/proxy/auth problems can make the whole cockpit empty.

## Network requirements

Every hop between browser and Server must allow:

- long-lived `GET` responses without a fixed `Content-Length`;
- response streaming with buffering disabled;
- idle/read timeouts longer than the 15-second keep-alive interval;
- no gzip/compression buffering for `text/event-stream`;
- no caching for `/api/v1/**/stream` responses;
- enough per-client and per-origin connections for multiple open console tabs;
- query-token fallback preserved when same-origin cookies or headers are unavailable.

Use `60s` as a minimum idle timeout. For operator consoles, `300s` to `3600s` is safer.

## nginx reverse proxy

Apply these settings to the Tikeo API location, or to a narrower stream location if your routing separates Web assets and API traffic.

```nginx
location /api/v1/ {
    proxy_pass http://tikeo-server:9090;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;

    # Required for SSE: do not buffer, cache, or compress streaming responses.
    proxy_buffering off;
    proxy_cache off;
    gzip off;

    # Keep streams open longer than every LB/WAF hop.
    proxy_read_timeout 3600s;
    proxy_send_timeout 3600s;

    # Avoid hop-by-hop connection header surprises with upstream HTTP/1.1.
    proxy_set_header Connection "";
}
```

Reload nginx only after validating the config:

```bash
nginx -t
nginx -s reload
```

Verify through the proxy:

```bash
curl -N https://tikeo.example.com/api/v1/workers/stream \
  -H "authorization: Bearer $TOKEN" \
  -H 'Accept: text/event-stream'
```

## Load balancers and CDNs

Recommended settings:

| Setting | Required behavior |
| --- | --- |
| Idle timeout | At least `60s`; prefer `300s+` for consoles. |
| Response buffering | Disabled for stream routes. |
| Upstream protocol | HTTP/1.1 is the least surprising; HTTP/2 is acceptable only if streaming flushes are preserved. |
| Health checks | Use `/readyz` or `/healthz`, never a stream route. |
| Backend draining | Avoid draining a backend while operators are watching active streams. |
| CDN cache | Bypass `/api/v1/` and all `/api/v1/**/stream` routes. |

Examples:

- AWS ALB: set `idle_timeout.timeout_seconds` to `300` or `3600`.
- Cloudflare or WAF/CDN products: confirm long-lived streaming HTTP responses are allowed; otherwise bypass proxying for stream routes.
- Envoy, HAProxy, and Traefik: disable response buffering and raise stream/idle timeouts for the Tikeo API route.

## WAF rules

SSE endpoints are authenticated long-lived `GET` requests with streaming JSON event frames. WAF rules must not:

- require `Content-Length` on responses;
- block `text/event-stream`;
- classify long-lived `GET` responses as slowloris or data exfiltration by default;
- strip or rewrite the `token` query parameter;
- buffer the response until a minimum body size is reached;
- inject JavaScript, CAPTCHA, or challenge pages into API responses.

Security checklist:

- Prefer same-origin Web/API deployments.
- Use HTTPS before exposing query-token fallback on a network.
- Redact query strings or `token` from access logs.
- Keep stream permissions aligned with the matching REST views.
- Rotate affected tokens if query strings were logged in a shared system.

## Kubernetes Ingress

### ingress-nginx

Use annotations like these on the Ingress serving the Tikeo API:

```yaml
metadata:
  annotations:
    nginx.ingress.kubernetes.io/proxy-buffering: "off"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-request-buffering: "off"
```

If your cluster blocks per-Ingress annotations, configure equivalent values at the IngressClass or controller ConfigMap level. Avoid rewrite rules that move `/api/v1/**/stream` away from the Tikeo Server.

### AWS ALB Ingress Controller

```yaml
metadata:
  annotations:
    alb.ingress.kubernetes.io/load-balancer-attributes: idle_timeout.timeout_seconds=3600
```

Set target group health checks to `/readyz`. If WAF is attached to the ALB, allow streaming `GET` responses for the Tikeo API stream routes.

### Other controllers

| Controller | Setting to check |
| --- | --- |
| Traefik | Forwarding and response timeouts; no buffering middleware for `/api/v1/`. |
| Envoy / Gateway API | Route timeout, stream idle timeout, and filters that buffer full responses. |
| NGINX Inc. controller | Equivalent read/send timeout and buffering controls for your controller version. |

## Verification runbook

1. Open browser DevTools and confirm the stream request stays pending.
2. Confirm response headers include `Content-Type: text/event-stream`.
3. Confirm the route is not cached and not served by a static CDN layer.
4. Compare direct Server access with proxied access:

   ```bash
   curl -N http://127.0.0.1:9090/api/v1/workers/stream \
     -H "authorization: Bearer $TOKEN" \
     -H 'Accept: text/event-stream'

   curl -N https://tikeo.example.com/api/v1/workers/stream \
     -H "authorization: Bearer $TOKEN" \
     -H 'Accept: text/event-stream'
   ```

5. Watch reconnect frequency. Reconnects at a fixed interval usually mean an idle timeout.
6. Inspect proxy/WAF/LB logs for `499`, `502`, `504`, `524`, `403`, or challenge-page responses.
7. Verify that `?token=...` is accepted when the browser uses `EventSource` and no `Authorization` header can be sent.
8. Confirm frontend and API origin/CORS policy if Web is not served from the same origin as the API.
9. Open Dashboard and verify the instance trend, Worker Mesh/capability coverage, and queue pressure panels change without a full page reload.

## Common symptoms

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| Events arrive in large batches | Proxy buffering or gzip buffering. | Disable buffering/compression for stream routes. |
| Browser reconnects every 30-60 seconds | Proxy, LB, WAF, or upstream idle timeout. | Raise all idle/read timeouts above the keep-alive interval. |
| Direct curl works, proxied curl hangs with no events | CDN or WAF response aggregation. | Bypass stream routes or disable aggregation. |
| `499`, `502`, or `504` during live views | Proxy closes long responses or upstream timeout mismatch. | Align nginx, LB, and Ingress timeouts; check Server health separately. |
| Browser gets HTML instead of event frames | WAF challenge, auth redirect, or frontend rewrite caught the API route. | Exclude `/api/v1/**/stream` from challenge/rewrite rules. |
| Token appears in logs | Query fallback logged by a network hop. | Redact query strings, rotate exposed tokens, prefer same-origin controlled logging. |

## Cleanup after testing

- Stop manual `curl -N` sessions with `Ctrl-C`.
- Close extra browser tabs that hold stream connections.
- Remove temporary access logs that captured query tokens, or redact them according to your retention policy.
- Revoke or rotate any token used in a shared proxy test if it may have been logged.

## Production checklist

Before exposing the Web console through a proxy or Ingress:

- `/readyz` and direct local stream checks pass.
- Stream routes bypass CDN/static caching.
- Buffering and gzip are disabled for stream responses.
- Idle/read timeouts are at least `60s`; `300s+` is preferred.
- WAF allows `text/event-stream` and long-lived authenticated `GET` requests.
- Query strings or `token` values are redacted from all access logs.
- Health checks use `/readyz` or `/healthz` only.
- Runbooks tell operators how to compare direct Server access with proxied access.

## Verify

After following the page, verify the result with the documented API, UI, build, smoke, or deployment checks. A valid verification includes the command that was run, the route or file that was inspected, and the observed status or artifact.

## Troubleshooting

When a step fails, first capture the exact command, response status, and Server log window. Then check authentication, namespace/app scope, Worker eligibility, storage readiness, and proxy behavior before changing production configuration.
