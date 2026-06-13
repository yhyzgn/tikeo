---
title: Python Worker SDK
description: Python SDK dependency coordinates, WorkerConfig defaults, minimal Worker, Management API helpers, and live verification runbook.
---

# Python Worker SDK

The Python SDK lives in `sdks/python/tikeo`, with package source under `sdks/python/tikeo/src/tikeo`. The runnable demo lives in `examples/python/worker-demo`. The SDK exposes Worker configuration, a gRPC Worker Tunnel client, task models, script runners, sandbox tool resolution, and Management API helpers.

## Dependency coordinates

`sdks/python/tikeo/pyproject.toml` declares:

| Field | Value |
| --- | --- |
| Package name | `tikeo` |
| Version in repo | `0.2.0` |
| Python baseline | `>=3.11` |
| Runtime deps | `grpcio>=1.76.0`, `grpcio-tools>=1.76.0`, `protobuf>=6.0.0`, `requests>=2.32.0` |
| Test extra | `pytest>=9.0.0` |

Install from PyPI when published:

```bash
python3 -m pip install "tikeo==${TIKEO_VERSION}"
```

Verify locally:

```bash
cd sdks/python/tikeo
python3 -m pip install -e '.[test]'
python3 -m pytest
```

## WorkerConfig defaults

`sdks/python/tikeo/src/tikeo/config.py` defines `local_config(endpoint, client_instance_id)`.

| Field | Default | Notes |
| --- | --- | --- |
| `endpoint` | caller-provided | Worker Tunnel endpoint. |
| `client_instance_id` | caller-provided | Stable client hint. |
| `namespace` | `default` | Demo overrides to `dev-alpha`. |
| `app` | `default` | Demo overrides to `orders`. |
| `name` | `client_instance_id` if blank | Operator-facing name. |
| `region` | `local` | Region metadata. |
| `version` | `dev` | Worker build/version. |
| `cluster` | `local` | Cluster metadata. |
| `capabilities` | empty list | Legacy metadata. |
| `labels` | empty dict | Demo adds `worker_pool`. |
| `structured` | empty `WorkerCapabilities` | Routing uses this. |
| `heartbeat_every` | `timedelta(seconds=10)` | Lease renewal cadence. |

Structured helpers include `add_tag`, `add_sdk_processor`, `add_script_runner`, and `add_plugin_processor`. Validation rejects blank endpoint/client/scope/name/cluster and non-positive heartbeat intervals.

## Minimal Worker

```python
import time
import tikeo

config = tikeo.local_config("http://127.0.0.1:9998", "python-worker-1")
config.namespace = "sdk-smoke"
config.app = "management"
config.add_sdk_processor("demo.echo")
config.labels["worker_pool"] = "python-blue"

client = tikeo.Client(config)

def process(task: tikeo.TaskContext) -> tikeo.TaskOutcome:
    task.log_info(f"python echo processor={task.processor_name} instance={task.instance_id}")
    return tikeo.succeeded("python echo processed")

while True:
    try:
        session = client.connect()
        stop = session.start_heartbeat()
        try:
            session.process_next(process)
        finally:
            stop.set()
            session.close()
    except Exception as exc:
        print(f"worker tunnel ended, reconnecting: {exc}")
        time.sleep(2)
```

`process_next(process, scripts)` should receive a `ScriptRunnerRegistry` only after registering real script runners. The Python SDK validates immutable script snapshots and SHA-256 digests before running script content.

## Demo environment variables

`examples/python/worker-demo/src/tikeo_python_worker_demo/__main__.py` uses:

| Variable | Default | Meaning |
| --- | --- | --- |
| `TIKEO_WORKER_ENDPOINT` | `http://127.0.0.1:9998` | Worker Tunnel endpoint. |
| `TIKEO_WORKER_CLIENT_INSTANCE_ID` | `python-worker-demo-local` | Stable client hint. |
| `TIKEO_WORKER_NAMESPACE` | `dev-alpha` | Demo namespace. |
| `TIKEO_WORKER_APP` | `orders` | Demo app. |
| `TIKEO_WORKER_CLUSTER` | `local` | Demo cluster. |
| `TIKEO_WORKER_REGION` | `local` | Demo region. |
| `TIKEO_WORKER_SDK_PROCESSORS` | `demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail,demo.exception` | Structured SDK processors. |
| `TIKEO_WORKER_POOL` | `python-blue` | `worker_pool` label. |
| `TIKEO_WORKER_SCRIPT_LANGUAGES` | `shell,python,javascript,typescript,powershell,php,groovy,rhai` | Candidate script languages. |
| `TIKEO_WORKER_SCRIPT_SANDBOX` | `auto` | `deno` for JS/TS, `srt` for native languages. |
| `TIKEO_SANDBOX_AUTO_INSTALL` | enabled unless disabled | Tool auto-install. |

Run dry-run:

```bash
cd examples/python/worker-demo
TIKEO_WORKER_DRY_RUN=1 python3 -m tikeo_python_worker_demo
```

Run live:

```bash
TIKEO_WORKER_CONNECT=1 \
TIKEO_WORKER_NAMESPACE=sdk-smoke \
TIKEO_WORKER_APP=management \
TIKEO_WORKER_SDK_PROCESSORS=demo.echo \
python3 -m tikeo_python_worker_demo
```

## Management API create + trigger

```python
import os
import tikeo

management = tikeo.ManagementClient(
    os.getenv("TIKEO_MANAGEMENT_ENDPOINT", "http://127.0.0.1:9090"),
    os.environ["TIKEO_MANAGEMENT_API_KEY"],
    "sdk-smoke",
    "management",
)
created = management.create_job(tikeo.api_job("python-echo-api", "demo.echo"))
instance = management.trigger_job(created.id, tikeo.api_trigger())

assert instance.trigger_type == "api"
assert instance.execution_mode == "single"
```

Broadcast is explicit:

```python
selector = tikeo.BroadcastSelectorRequest(
    tags=["manual-demo"],
    region="local",
    cluster="local",
    labels={"worker_pool": "python-blue"},
)
request = tikeo.broadcast_api_trigger(selector)
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

Expected acceptance evidence includes an online worker with the requested structured processor, an API-triggered instance with `executionMode=single`, task logs from the Worker, and a successful processor message. Missing sandbox tools or unsupported processors must fail closed and be visible in task/diagnostic logs.

## Failure and exception demos

All language demos now separate business failure from runtime exceptions. `demo.fail` returns a normal failed `TaskOutcome` and is used to verify business-rule failure handling. `demo.exception` throws, panics, raises, or returns a processor error so the SDK can capture a real runtime stack trace and send it as task logs while still reporting a failed task result. Use both processors during acceptance: the first proves expected business failure semantics; the second proves operator-visible stack traces survive the Worker Tunnel and Notification Center trace page.

## Capability discipline

The dispatch contract uses structured capabilities, not folklore or only string naming conventions. A Worker should advertise SDK processors, plugin processors, script runners, labels, and tags only when the runtime can really execute them. Do not advertise SQL, shell, Python, Node.js, WASM, SRT, Deno, Docker, or Podman support just because a package exists; advertise it after the demo or service has resolved the tool and can fail safely.

## Operational notes

A Python Worker is often packaged inside an application virtual environment or container image. Keep `grpcio`, `protobuf`, and `requests` versions pinned by your service lockfile, and avoid importing the demo package into production services. Production code should import `tikeo`, construct `WorkerConfig`, and register only the processors and script runners owned by that service.

The demo reconnect loop mirrors the intended service behavior: `client.connect()` creates a session, `session.start_heartbeat()` renews the lease, `session.process_next(...)` waits for one dispatch and returns a `TaskOutcome`, and `session.close()` sends unregister. If the stream ends, the demo waits and reconnects. Keep that lifecycle visible in logs so operators can distinguish a normal Server rollout from a processor exception.

For scripts, `ScriptRunnerRegistry.add_capabilities(config)` only advertises runners whose `advertise_capability()` returns true. `UnavailableScriptRunner` returns false and can still produce a clear failure if deliberately registered for testing. This is important for Python deployments because local shelling-out is easy; Tikeo expects missing tools to fail closed, not to be advertised as working capability.

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
