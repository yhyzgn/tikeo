---
title: Go Worker SDK
description: Verified Go SDK and Worker demo entry points.
---

# Go Worker SDK

The Go SDK lives under `sdks/go/tikeo`, and the runnable worker demo lives under `examples/go/worker-demo`.


## Install from the Go module proxy

Replace `${TIKEO_VERSION}` with the version shown by the top README `Go SDK` badge. Go commands use tag syntax, so include the leading `v` as `v${TIKEO_VERSION}`.

```bash
go get github.com/yhyzgn/tikeo/sdks/go/tikeo@v${TIKEO_VERSION}
```

```go
import "github.com/yhyzgn/tikeo/sdks/go/tikeo"
```

## Verify the SDK

```bash
cd sdks/go/tikeo
go test ./... -count=1
```

## Verify the demo

```bash
cd examples/go/worker-demo
go test ./... -count=1
```

Go workers should advertise only capabilities backed by real runtime support. Unsupported sandbox runners must fail closed instead of appearing as available capabilities.


## Management API create + trigger

The Go management client is implemented in `sdks/go/tikeo/management.go`. It is scoped to one namespace/app and authenticates with `x-tikeo-api-key`, typically read from `TIKEO_MANAGEMENT_API_KEY`; do not pass a human OIDC session or UI bearer token into SDK workers. The `APIJob` helper creates an API-scheduled processor job, while `APITrigger` sends `triggerType=api` with the default `executionMode=single`.

```go
package main

import (
    "context"
    "os"

    tikeo "github.com/yhyzgn/tikeo/sdks/go/tikeo"
)

func createAndTrigger(ctx context.Context) error {
    endpoint := os.Getenv("TIKEO_MANAGEMENT_ENDPOINT")
    if endpoint == "" {
        endpoint = "http://127.0.0.1:9090"
    }
    client := tikeo.NewManagementClient(
        endpoint,
        os.Getenv("TIKEO_MANAGEMENT_API_KEY"),
        "dev-alpha",
        "orders",
    )

    created, err := client.CreateJob(ctx, tikeo.APIJob("go-echo-api", "demo.echo"))
    if err != nil {
        return err
    }
    instance, err := client.TriggerJob(ctx, created.ID, tikeo.APITrigger())
    if err != nil {
        return err
    }
    if instance.TriggerType != "api" || instance.ExecutionMode != "single" {
        panic("unexpected trigger response")
    }
    return nil
}
```

Broadcast fan-out is explicit. `BroadcastAPITrigger` serializes `executionMode=broadcast` plus `broadcastSelector`; keep this separate from the single-worker default so accidental API triggers do not run on every matching worker.

```go
broadcast := tikeo.BroadcastAPITrigger(&tikeo.BroadcastSelectorRequest{
    Tags:   []string{"manual-demo"},
    Region: "us-east-1",
    Labels: map[string]string{"worker_pool": "go-blue"},
})
_, err := client.TriggerJob(ctx, created.ID, broadcast)
```


## Source-backed reference links

Keep SDK helper docs anchored to source-derived API and protocol references:

- Create helper endpoint: [`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- Trigger helper endpoint: [`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- Instance polling endpoint: [`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- Instance log endpoint: [`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker dispatch message: [`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## Minimal worker mental model

The Go SDK follows the same Worker Tunnel model as Rust and Java. A worker connects out to the Server, registers metadata, heartbeats, receives dispatches, and reports logs/results back through the tunnel.

## Capability discipline

Go workers should advertise structured processor and script capabilities only when backed by real runtime support. If a runner is unavailable, the worker should expose a safe error boundary rather than pretending the capability exists.

## Evaluation checklist

- Run `go test ./... -count=1` in both SDK and demo directories.
- Start the Server locally and connect a Go worker in live mode when validating tunnel behavior.
- Confirm session visibility survives expected Server-side persistence boundaries.
- Trigger a job that maps to the Go processor binding.
- Inspect logs, result payload, and audit evidence.

## Production notes

Use Go workers when teams want lightweight static binaries or Go-native integrations. Keep deployment identity explicit through worker pool and capability metadata.

## Version and packaging notes

Repository builds currently exercise the Go surface with the Go toolchain declared by CI and README badges. Keep `go.mod` and demo documentation aligned before publishing external examples. For containerized workers, prefer small images that include only the worker binary, configuration, and trusted certificates needed to dial the Tikeo tunnel.
