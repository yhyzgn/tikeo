---
title: Node.js Worker SDK
description: Node.js SDK dependency coordinates, WorkerConfig defaults, minimal Worker, Management API helpers, and live verification runbook.
---

# Node.js Worker SDK

The Node.js SDK lives in `sdks/nodejs/tikeo`; the runnable Bun demo lives in `examples/nodejs/worker-demo`. Repository commands use Bun, and the published package is `@yhyzgn/tikeo`.


Shared SDK/API contract: see [SDK and API integration guide](../integrations/sdk-and-api) for common concepts, unified configuration parameters, Management API semantics, Worker connection parameters, trigger types, errors/retries, and the language difference table. This language page stays focused on installation, minimal Worker code, exception behavior, and Management client syntax.

## Dependency coordinates

`sdks/nodejs/tikeo/package.json` declares:

| Field | Value |
| --- | --- |
| Package | `@yhyzgn/tikeo` |
| Version placeholder | `${TIKEO_VERSION}` from the README/top package badge or release tag. |
| Module type | ESM (`type=module`) |
| Runtime baseline | Node.js `>=24.0.0` |
| Main | `dist/index.js` |
| Bun export | `src/index.ts` |
| Runtime deps | `@grpc/grpc-js`, `@grpc/proto-loader` |
| Repository runner | Bun |

Install from npm when published:

```bash
bun add @yhyzgn/tikeo@${TIKEO_VERSION}
# or for non-repo consumers:
npm install @yhyzgn/tikeo@${TIKEO_VERSION}
```

Verify locally:

```bash
cd sdks/nodejs/tikeo
bun install --frozen-lockfile
bun test
bun run build
```

## WorkerConfig defaults

`sdks/nodejs/tikeo/src/config.ts` defines `new WorkerConfig(input)` and `localConfig(endpoint, clientInstanceId)`.

| Field | Default | Notes |
| --- | --- | --- |
| `endpoint` | required | Worker Tunnel endpoint. |
| `clientInstanceId` | required | Stable client hint. |
| `namespace` | `default` | Demo overrides to `dev-alpha`. |
| `app` | `default` | Demo overrides to `orders`. |
| `name` | `clientInstanceId` | Operator-facing name. |
| `region` | `local` | Region metadata. |
| `version` | `dev` | Worker build/version. |
| `cluster` | `local` | Cluster metadata. |
| `capabilities` | `[]` | Legacy metadata. |
| `labels` | `{}` | Demo adds `worker_pool`. |
| `structured.tags` | `[]` | Operator tags. |
| `structured.sdkProcessors` | `[]` | Dispatch processors. |
| `structured.scriptRunners` | `[]` | Script language/backend pairs. |
| `structured.pluginProcessors` | `[]` | Plugin type/name pairs. |
| `heartbeatEveryMs` | `10000` | Lease renewal cadence. |

The `validate()` method rejects blank required fields and non-positive heartbeat intervals. `normalize()` deduplicates legacy capabilities, tags, SDK processors, and plugin processor names.

## Minimal Worker

```typescript
import { Client, installConsoleTaskLogBridge, localConfig, type TaskContext, type TaskOutcome } from "@yhyzgn/tikeo";

const config = localConfig("http://127.0.0.1:9998", "nodejs-worker-1");
config.namespace = "sdk-smoke";
config.app = "management";
config.addSDKProcessor("demo.echo");
config.labels.worker_pool = "nodejs-blue";

installConsoleTaskLogBridge(); // Mirrors console.* only while a Tikeo task scope is active.
const client = new Client(config);

async function process(task: TaskContext): Promise<TaskOutcome> {
  console.info(`nodejs echo processor=${task.processorName} instance=${task.instanceId}`);
  return { success: true, message: "nodejs echo processed" };
}

while (true) {
  try {
    const session = await client.connect();
    const stop = session.startHeartbeat();
    try {
      await session.processNext(process);
    } finally {
      stop();
      session.close();
    }
  } catch (error) {
    console.warn(`worker tunnel ended, reconnecting: ${(error as Error).message}`);
    await new Promise((resolve) => setTimeout(resolve, 2_000));
  }
}
```

## Task logging bridge

Use ordinary application logging in processors. `installConsoleTaskLogBridge()` uses Node `AsyncLocalStorage`, so `console.info/error/warn/log` lines are mirrored to the current job instance only during `session.processNext(...)`. Logs outside a task remain normal process logs. `TaskContext.logInfo/logError` remains as a low-level fallback for custom logger integrations.

## Demo environment variables

`examples/nodejs/worker-demo/src/main.ts` uses:

| Variable | Default | Meaning |
| --- | --- | --- |
| `TIKEO_WORKER_ENDPOINT` | `http://127.0.0.1:9998` | Worker Tunnel endpoint. |
| `TIKEO_WORKER_CLIENT_INSTANCE_ID` | `nodejs-worker-demo-local` | Stable client hint. |
| `TIKEO_WORKER_NAMESPACE` | `dev-alpha` | Demo namespace. |
| `TIKEO_WORKER_APP` | `orders` | Demo app. |
| `TIKEO_WORKER_CLUSTER` | `local` | Demo cluster. |
| `TIKEO_WORKER_REGION` | `local` | Demo region. |
| `TIKEO_WORKER_SDK_PROCESSORS` | `demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail,demo.exception` | Structured SDK processors. |
| `TIKEO_WORKER_POOL` | `nodejs-blue` | `worker_pool` label. |
| `TIKEO_WORKER_SCRIPT_LANGUAGES` | `shell,python,javascript,typescript,powershell,php,groovy,rhai` | Candidate script languages. |
| `TIKEO_WORKER_SCRIPT_SANDBOX` | `auto` | `deno` for JS/TS, `srt` for native languages. |
| `TIKEO_WORKER_DRY_RUN` / `TIKEO_WORKER_CONNECT=0` | dry-run | Avoids live tunnel. |

Run dry-run:

```bash
cd examples/nodejs/worker-demo
bun install --frozen-lockfile
TIKEO_WORKER_DRY_RUN=1 bun start
```

Run live:

```bash
TIKEO_WORKER_CONNECT=1 \
TIKEO_WORKER_NAMESPACE=sdk-smoke \
TIKEO_WORKER_APP=management \
TIKEO_WORKER_SDK_PROCESSORS=demo.echo \
TIKEO_ENABLE_PLUGIN_SQL=0 \
TIKEO_SANDBOX_AUTO_INSTALL=0 \
bun start
```

## Management API create + trigger

```typescript
import { ManagementClient, apiJob, apiTrigger, broadcastApiTrigger, type BroadcastSelectorRequest } from "@yhyzgn/tikeo";

const management = new ManagementClient(
  process.env.TIKEO_MANAGEMENT_ENDPOINT ?? "http://127.0.0.1:9090",
  process.env.TIKEO_MANAGEMENT_API_KEY ?? "",
  "sdk-smoke",
  "management",
);

const created = await management.createJob(apiJob("nodejs-echo-api", "demo.echo"));
const instance = await management.triggerJob(created.id, apiTrigger());

if (instance.triggerType !== "api" || instance.executionMode !== "single") {
  throw new Error("unexpected trigger response");
}
```

Broadcast is explicit:

```typescript
const selector: BroadcastSelectorRequest = {
  tags: ["manual-demo"],
  region: "local",
  cluster: "local",
  labels: { worker_pool: "nodejs-blue" },
};
await management.triggerJob(created.id, broadcastApiTrigger(selector));
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
