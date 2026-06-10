# Dashboard user guide

The Dashboard is the first operating surface after login. It is implemented by `web/src/pages/Dashboard.tsx` and intentionally summarizes only data that the Web console already retrieves from the Management API and Server-Sent Events streams. Use it as a live triage board, not as a separate configuration source.

## Source-backed data paths

`web/src/pages/Dashboard.tsx` loads Jobs through `/api/v1/jobs`, Worker state through `/api/v1/workers`, and per-job instance pages through `/api/v1/jobs/{job}/instances`. It also listens to `/api/v1/instances/stream` and `/api/v1/workers/stream` so the cards refresh when instances or workers change. The backend source for platform health and capacity also exposes `/api/v1/metrics/summary` and `/api/v1/cluster`; those endpoints are useful when comparing dashboard symptoms with raw API evidence.

## What the cards mean

The visible cards count total jobs, enabled jobs, pending instances, online workers, and broadcast instances. These values come from the same `JobSummary`, `JobInstanceSummary`, and `WorkerListResponse` types used elsewhere in the console. If a count looks wrong, open Jobs, Instances, or Workers first before changing configuration.

## Operator workflow

Start with the online Worker count. If it is zero, go to Workers and confirm Worker Tunnel sessions. If workers are online but pending instances keep growing, go to Jobs to inspect processor names, worker pools, scheduling advice, and trigger mode. If failed or partial broadcast executions appear, open Instances and review logs per execution node.

## Boundaries and caveats

The Dashboard does not create, update, retry, cancel, or approve anything. It is a read surface backed by existing APIs and streams. When stream frames are malformed or temporarily unavailable, `Dashboard.tsx` silently falls back to periodic refresh, so stale numbers usually indicate connectivity or API errors rather than a hidden dashboard-only state.
