---
title: Server, Worker, and Worker Tunnel
description: The outbound-only Worker Tunnel model that differentiates Tikeo from server-calls-executor schedulers.
---

# Server, Worker, and Worker Tunnel

Tikeo's key runtime boundary is the Worker Tunnel.

```text
Worker process  ── outbound gRPC/HTTP2 tunnel ──>  Tikeo Server
       ▲                                              │
       └──────── dispatch / cancel / logs / result ───┘
```

## Why outbound-only matters

Workers may run in Kubernetes, private VPCs, separate clusters, or locked-down networks. Tikeo does not require business workers to expose inbound execution ports. The worker registers, heartbeats, receives dispatches, reports logs, returns results, and unregisters through the long-lived tunnel.

## Identity and fencing

The Server assigns authoritative worker identity during registration. Session generation and fencing tokens prevent stale workers from reporting results for a replaced logical worker.

## Operational visibility

Worker sessions and capability snapshots are persisted, so restarts can preserve visibility evidence instead of relying only on memory state.

## Security boundary

The Server schedules and governs. User code, dynamic scripts, sandbox runners, HTTP calls, SQL processors, and plugin processors execute on Workers or controlled runtimes, not inside the Server process.
