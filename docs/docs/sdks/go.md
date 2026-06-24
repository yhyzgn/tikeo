---
title: Go Worker SDK
description: Go SDK dependency coordinates, WorkerConfig defaults, minimal Worker, Management API helpers, and live verification runbook.
---

# Go Worker SDK

The Go SDK lives in `sdks/go/tikeo`. Its module path is `github.com/yhyzgn/tikeo/sdks/go/tikeo`; the runnable demo is `examples/go/worker-demo`. The SDK exposes Worker configuration, a Worker Tunnel client, structured capabilities, script runner helpers, task models, and Management client helpers.


Shared SDK/API contract: see [SDK and API integration guide](../integrations/sdk-and-api) for common concepts, unified configuration parameters, Management API semantics, Worker connection parameters, trigger types, errors/retries, and the language difference table. This language page stays focused on installation, minimal Worker code, exception behavior, and Management client syntax.

## Dependency coordinates

`sdks/go/tikeo/go.mod` declares:

| Field | Value |
| --- | --- |
| Module | `github.com/yhyzgn/tikeo/sdks/go/tikeo` |
| Go baseline | `1.26` |
| gRPC | `google.golang.org/grpc v1.81.0` |
| Protobuf | `google.golang.org/protobuf v1.36.11` |

Install from a tagged repository release:

```bash
go get github.com/yhyzgn/tikeo/sdks/go/tikeo@${TIKEO_VERSION}
```

Verify locally:

```bash
cd sdks/go/tikeo
go test ./... -count=1
```

## WorkerConfig defaults

`sdks/go/tikeo/config.go` defines `LocalConfig(endpoint, clientInstanceID)`.

| Field | Default from helper | Notes |
| --- | --- | --- |
| `Endpoint` | caller-provided | Worker Tunnel endpoint. |
| `ClientInstanceID` | caller-provided | Stable client hint. |
| `Namespace` | `default` | Demo overrides to `dev-alpha`. |
| `App` | `default` | Demo overrides to `orders`. |
| `Name` | `clientInstanceID` | Operator-facing name. |
| `Region` | `local` | Region metadata. |
| `Version` | `dev` | Worker build/version. |
| `Cluster` | `local` | Cluster metadata. |
| `Capabilities` | empty | Legacy metadata. |
| `Labels` | empty map | Demo adds `worker_pool`. |
| `Structured` | empty `WorkerCapabilities` | Routing uses this. |
| `HeartbeatEvery` | `10 * time.Second` | Lease renewal cadence. |

Use `AddTag`, `AddSDKProcessor`, `AddScriptRunner`, and `AddPluginProcessor` for structured registration.

## Minimal Worker

```go
package main

import (
  "context"
  "log"
  "log/slog"

  tikeo "github.com/yhyzgn/tikeo/sdks/go/tikeo"
)

func main() {
  cfg := tikeo.LocalConfig("http://127.0.0.1:9998", "go-worker-1")
  cfg.Namespace = "sdk-smoke"
  cfg.App = "management"
  cfg.AddSDKProcessor("demo.echo")
  cfg.Labels["worker_pool"] = "go-blue"

  client, err := tikeo.NewClient(cfg)
  if err != nil { log.Fatal(err) }

  processor := tikeo.TaskProcessorFunc(func(ctx context.Context, task tikeo.TaskContext) (tikeo.TaskOutcome, error) {
    slog.New(tikeo.TaskSlogHandler{}).InfoContext(ctx, "go echo processor", "processor", task.ProcessorName, "instance", task.InstanceID)
    return tikeo.TaskOutcome{Success: true, Message: "go echo processed"}, nil
  })

  for {
    session, err := client.Connect(context.Background())
    if err != nil { log.Printf("connect failed: %v", err); continue }
    stop := session.StartHeartbeat(context.Background())
    _, _ = session.ProcessNext(context.Background(), processor)
    stop()
    _ = session.Close()
  }
}
```

Use normal `slog` in processors with `TaskSlogHandler`; it reads the task scope from `context.Context` and includes structured fields in the instance log message. `NewTaskLogger(ctx, ...)` supports legacy `log.Logger` code. `TaskContext.LogInfo/LogError` remains a direct fallback.

Keep the reconnect loop conservative in services; a Worker Tunnel can close due to Server restart, network churn, fencing, or rollout.

## Demo environment variables

`examples/go/worker-demo/main.go` uses:

| Variable | Default | Meaning |
| --- | --- | --- |
| `TIKEO_WORKER_ENDPOINT` | `http://127.0.0.1:9998` | Worker Tunnel endpoint. |
| `TIKEO_WORKER_CLIENT_INSTANCE_ID` | `go-worker-demo-local` | Stable client hint. |
| `TIKEO_WORKER_NAMESPACE` | `dev-alpha` | Demo namespace. |
| `TIKEO_WORKER_APP` | `orders` | Demo app. |
| `TIKEO_WORKER_CLUSTER` | `local` | Demo cluster. |
| `TIKEO_WORKER_REGION` | `local` | Demo region. |
| `TIKEO_WORKER_SDK_PROCESSORS` | `demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail,demo.exception` | Structured SDK processors. |
| `TIKEO_WORKER_POOL` | `go-blue` | `worker_pool` label. |
| `TIKEO_WORKER_DRY_RUN` / `TIKEO_WORKER_CONNECT=0` | dry-run | Avoids live tunnel. |
| `TIKEO_WORKER_ONESHOT` | unset | Exit after one task. |

Run dry-run:

```bash
cd examples/go/worker-demo
TIKEO_WORKER_DRY_RUN=1 go run .
```

Run live:

```bash
TIKEO_WORKER_CONNECT=1 \
TIKEO_WORKER_NAMESPACE=sdk-smoke \
TIKEO_WORKER_APP=management \
TIKEO_WORKER_SDK_PROCESSORS=demo.echo \
go run .
```

## Management API create + trigger

```go
package main

import (
  "context"
  "os"

  tikeo "github.com/yhyzgn/tikeo/sdks/go/tikeo"
)

func main() {
  client := tikeo.NewManagementClient(
    env("TIKEO_MANAGEMENT_ENDPOINT", "http://127.0.0.1:9090"),
    os.Getenv("TIKEO_MANAGEMENT_API_KEY"),
    "sdk-smoke",
    "management",
  )
  created, err := client.CreateJob(context.Background(), tikeo.APIJob("go-echo-api", "demo.echo"))
  if err != nil { panic(err) }
  instance, err := client.TriggerJob(context.Background(), created.ID, tikeo.APITrigger())
  if err != nil { panic(err) }
  if instance.TriggerType != "api" || instance.ExecutionMode != "single" { panic("unexpected trigger") }
}

func env(key, fallback string) string { if v := os.Getenv(key); v != "" { return v }; return fallback }
```

Broadcast is explicit:

```go
selector := &tikeo.BroadcastSelectorRequest{
  Tags: []string{"manual-demo"},
  Region: "local",
  Cluster: "local",
  Labels: map[string]string{"worker_pool": "go-blue"},
}
request := tikeo.BroadcastAPITrigger(selector)
```

## Management client credentials

All SDK Management clients use app-scoped service credentials. They send the `x-tikeo-api-key` header, normally sourced from `TIKEO_MANAGEMENT_API_KEY`. Do not confuse this key with a human bearer token from `/api/v1/auth/login`, and do not reuse browser sessions or OIDC provider tokens in SDK services.

The common create+trigger default is:

| Field | Default helper behavior |
| --- | --- |
| Job schedule | `scheduleType=api` |
| Job enabled | `true` |
| Retry policy | `enabled=true`, `maxAttempts=3`, `initialDelaySeconds=5`, `backoffMultiplier=2`, `maxDelaySeconds=60` |
| Trigger source | `triggerType=api` |
| Trigger execution mode | `executionMode=single` |
| Broadcast | Opt-in only through explicit broadcast helper and `broadcastSelector` |

## Operator-verified reference links

Keep SDK helper docs anchored to operator-verified API and protocol references:

- Create helper endpoint: [`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- Trigger helper endpoint: [`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- Instance polling endpoint: [`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- Instance log endpoint: [`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker dispatch message: [`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## Live verification runbook

1. Start the Server with `cargo run --bin tikeo -- serve --config config/dev.toml`.
2. Bootstrap an Owner or login to an existing local Owner.
3. Create namespace/app/worker pool, service account, and SDK API key as shown in the quickstart.
4. Start the language demo Worker with matching namespace/app and `TIKEO_WORKER_CONNECT=1` when the demo supports live mode.
5. Create and trigger an API job through the language Management client.
6. Inspect `/api/v1/workers`, `/api/v1/instances`, instance logs, and audit logs.
7. Preserve smoke evidence. For a maintained end-to-end proof, run `TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh`.

Sandbox tooling note: default mode may reuse host PATH tools while keeping task cwd/env under sandbox runtime directories. Set `TIKEO_SANDBOX_REQUIRE_MANAGED_TOOLS=1` to skip host PATH tools/interpreters and require managed sandbox-tools binaries.

Expected acceptance evidence includes an online worker with the requested structured processor, an API-triggered instance with `executionMode=single`, task logs from the Worker, and a successful processor message. Missing sandbox tools or unsupported processors must fail closed and be visible in task/diagnostic logs.

## Failure and exception demos

All language demos now separate business failure from runtime exceptions. `demo.fail` returns a normal failed `TaskOutcome` and is used to verify business-rule failure handling. `demo.exception` throws, panics, raises, or returns a processor error so the SDK can capture a real runtime stack trace and send it as task logs while still reporting a failed task result. Use both processors during acceptance: the first proves expected business failure semantics; the second proves operator-visible stack traces survive the Worker Tunnel and Notification Center trace page.

## Capability discipline

The dispatch contract uses structured capabilities, not folklore or only string naming conventions. A Worker should advertise SDK processors, plugin processors, script runners, labels, and tags only when the runtime can really execute them. Do not advertise SQL, shell, Python, Node.js, WASM, SRT, Deno, Docker, or Podman support just because a package exists; advertise it after the demo or service has resolved the tool and can fail safely.

## Operational notes

A Go service should usually construct `WorkerConfig` from its own application configuration rather than reading every environment variable directly in business code. Keep the mapping explicit: endpoint from service discovery or a Secret-backed config map, namespace/app from the deployment environment, `ClientInstanceID` from a stable pod/VM identity if you want reconnect correlation, and `Labels["worker_pool"]` from your capacity planning model.

The demo's `runWorkerSession` pattern intentionally reconnects after `Connect` or `ProcessNextWithScriptRunners` errors. Keep that behavior in production services unless your supervisor is responsible for restart policy. A Worker Tunnel can close during Server restarts, load balancer rotation, lease fencing, TLS certificate reloads, or network churn; treating every close as a permanent crash makes rollouts noisier than necessary.

For script support, `examples/go/worker-demo/main.go` resolves SRT and ripgrep for native scripts and Deno for JavaScript/TypeScript when `TIKEO_WORKER_SCRIPT_SANDBOX=auto`. If those tools are missing, the demo skips advertising that script runner unless `TIKEO_ENABLE_UNAVAILABLE_SCRIPT_ADAPTERS` is enabled for explicit fail-closed demonstration. Production Workers should follow the same rule: no capability advertisement without a working runtime.

## Prerequisites

Use the setup, authentication, and access requirements described in this page before running any command. For local examples, start the Server with `config/dev.toml`, use `127.0.0.1` as the client host, and keep tokens in shell variables rather than pasted into files.

## Verify

After following the page, verify the result with the documented API, UI, build, smoke, or deployment checks. A valid verification includes the command that was run, the route or file that was inspected, and the observed status or artifact.

## Troubleshooting

When a step fails, first capture the exact command, response status, and Server log window. Then check authentication, namespace/app scope, Worker eligibility, storage readiness, and proxy behavior before changing production configuration.

## Production checklist

- [ ] Secrets are referenced through environment or platform secret mechanisms and are not written into examples.
- [ ] Commands have been adapted from local `127.0.0.1` to the real host, TLS, and authentication model.
- [ ] Rollback and evidence collection are documented for the changed surface.
- [ ] Operators can repeat the verification without private shell history or hidden state.
