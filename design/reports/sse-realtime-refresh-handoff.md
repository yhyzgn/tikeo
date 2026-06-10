# SSE realtime refresh handoff

Date: 2026-06-10

## Current objective

Move volatile console data from manual refresh / stale list state toward Server-Sent Events (SSE), while keeping safe polling fallbacks for deployments where nginx, LB, WAF, or ingress layers buffer or drop long-lived streams.

## Completed in this handoff chain

### Backend SSE surfaces

- `GET /api/v1/workers/stream`
  - Emits `workers.snapshot`.
  - Used by Workers page, Dashboard, and Jobs processor choices.
- `GET /api/v1/dispatch-queue/stream`
  - Emits `dispatchQueue.snapshot`.
  - Used by Dispatch Queue page.
- `GET /api/v1/instances/{instance}/logs/stream`
  - Emits `instance.snapshot` and `instance.log`.
  - Used by Instance log drawer.
- `GET /api/v1/instances/stream`
  - Emits `instances.snapshot` with visible jobs, visible instances, and grouped attempts.
  - Used by Instances page and Dashboard.
- `GET /api/v1/events/instances/{id}/stream`
  - Existing workflow instance event stream.
  - Used by Workflow page.

### Frontend pages now using SSE

- `web/src/pages/WorkersPage.tsx`
  - `workerStreamUrl()` updates worker list/history.
- `web/src/pages/DispatchQueuePage.tsx`
  - `dispatchQueueStreamUrl()` updates queue counts/items.
- `web/src/pages/InstancesPage.tsx`
  - `instanceListStreamUrl()` updates list/attempts.
  - `instanceLogStreamUrl(instanceId)` updates drawer snapshot/logs.
  - Keeps `window.setInterval(..., 3000)` fallback while active.
- `web/src/pages/Dashboard.tsx`
  - `instanceListStreamUrl()` updates job/instance metrics.
  - `workerStreamUrl()` updates online worker metric.
  - Keeps `window.setInterval(..., 3000)` fallback while active.
- `web/src/pages/JobsPage.tsx`
  - `workerStreamUrl()` updates worker-derived SDK processor options.

### Documentation already updated earlier

- README/docs mention SSE deployment caveats for nginx, LB, WAF, and Kubernetes ingress.
- Java SDK docs were expanded separately and pushed.

## Latest change in progress before this handoff

JobsPage now subscribes to `workerStreamUrl()` so newly started/stopped workers update SDK processor choices without a page refresh.

Files touched:

- `web/src/pages/JobsPage.tsx`
- `web/src/pages/__tests__/JobsPage.test.tsx`
- this handoff doc

Verification already run before creating this doc:

```bash
cd web && bun test src/pages/__tests__/JobsPage.test.tsx
cd web && bun run typecheck && bun run test
```

Observed result: `121 pass, 0 fail` for the full web test suite.

## Recommended next step

Extract a shared SSE hook to reduce repeated `EventSource` wiring and make future reconnect/fallback policy consistent.

Suggested shape:

```ts
useSseSnapshot<T>({
  active,
  url,
  eventName,
  onSnapshot,
  onMalformedFrame,
  fallbackIntervalMs,
  fallbackRefresh,
});
```

Candidate pages to migrate first:

1. `InstancesPage.tsx`
2. `Dashboard.tsx`
3. `WorkersPage.tsx`
4. `DispatchQueuePage.tsx`
5. `JobsPage.tsx`

Acceptance criteria:

- Existing behavior unchanged.
- Each page closes streams on unmount / inactive route.
- Fallback polling remains only where currently present or explicitly needed.
- Tests assert the hook is used and existing URL/event names remain intact.
- `cd web && bun run typecheck && bun run test` passes.

## Remaining work backlog

### P0 / near-term

1. **Shared SSE hook**
   - Remove duplicated `new EventSource(...)`, JSON parse, close cleanup, and fallback timer code.
   - Centralize malformed-frame handling and optional `onerror` fallback behavior.

2. **Runtime smoke test through real proxy**
   - Validate streams through at least one nginx config and one Kubernetes ingress setup.
   - Confirm headers: `Content-Type: text/event-stream`, no buffering, no gzip/chunk buffering surprises.
   - Confirm idle keepalive survives expected LB timeout window.

3. **Docs cross-link**
   - Add a concise “Realtime console streams” page/table in docs site listing:
     - endpoint
     - event name
     - consuming page
     - fallback behavior
     - required proxy settings

### P1

4. **SSE reconnection state**
   - For streams with monotonically ordered events, use `Last-Event-ID` where meaningful.
   - Current snapshot streams tolerate reconnect by re-sending full snapshots; log stream already tracks sequence internally.

5. **Fallback health indicator**
   - Add small non-blocking UI indicator when a page is using fallback polling because SSE failed.
   - Avoid noisy toasts on transient reconnects.

6. **Scopes/API Keys refresh review**
   - Check if `ScopesPage` / `ApiKeysPage` should refresh worker pool / service-account binding data after scope mutations or if manual refresh is enough.
   - Do not add SSE unless data is volatile in practice.

### P2

7. **Server-side stream tests for `/api/v1/instances/stream`**
   - Current server compiles and full server tests pass, but there is not yet a focused HTTP SSE test that opens the stream and asserts the first `instances.snapshot` frame.

8. **OpenAPI / API docs for stream endpoints**
   - Some SSE endpoints are implementation routes without detailed utoipa docs.
   - Add documented response/event contracts if the public API is considered stable.

## Known local caveat

` tikeo-dev.db` is modified locally by development/test activity and should continue to be excluded from commits unless explicitly requested.
