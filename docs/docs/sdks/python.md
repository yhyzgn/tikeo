---
title: Python Worker SDK
description: Verified Python SDK and Worker demo entry points.
---

# Python Worker SDK

The Python SDK lives under `sdks/python/tikeo`, and the runnable worker demo lives under `examples/python/worker-demo`. It is intended for automation workers, data-oriented processors, and teams that already operate Python runtime environments.

## Runtime requirement

The package declares `requires-python = ">=3.11"` and CI verifies the Python surface with Python 3.12. Keep the docs, `pyproject.toml`, CI matrix, and README runtime badge aligned whenever the baseline changes.


## Install from PyPI

Replace `${TIKEO_VERSION}` with the version shown by the top README `Python SDK` badge. PyPI uses the plain version string without a leading `v`.

```bash
python -m pip install "tikeo==${TIKEO_VERSION}"
```

```python
from tikeo import Client, local_config
```

## Verify the SDK

```bash
cd sdks/python/tikeo
python -m pip install -e .[test]
python -m pytest
```

The SDK depends on `grpcio`, `grpcio-tools`, `protobuf`, and `requests` for the Worker Tunnel and management helper surface.

## Verify the demo

```bash
cd examples/python/worker-demo
python -m pip install -e ../../../sdks/python/tikeo
python -m pip install -e .
TIKEO_WORKER_DRY_RUN=1 python -m tikeo_python_worker_demo
```

Dry-run mode is useful for checking local packaging and capability declarations without requiring a running Server.

## Live-mode expectations

Live mode defaults to `http://127.0.0.1:9998`, registers under the development scope used by the demo, advertises structured SDK/plugin/script capabilities, and uses sandbox auto-resolution for supported runners. When running live, start the Server first and verify the worker appears in the Web console.


## Management API create + trigger

The Python management helpers live in `sdks/python/tikeo/src/tikeo/management.py`. They use a namespace/app-scoped API key header (`x-tikeo-api-key`) sourced from a Secret such as `TIKEO_MANAGEMENT_API_KEY`; they are not a wrapper around a human login session. `api_job` creates a job with `scheduleType=api`, and `api_trigger` sends `triggerType=api` plus the default `executionMode=single`.

```python
import os
import tikeo

management = tikeo.ManagementClient(
    os.getenv("TIKEO_MANAGEMENT_ENDPOINT", "http://127.0.0.1:9090"),
    os.environ["TIKEO_MANAGEMENT_API_KEY"],
    "dev-alpha",
    "orders",
)

created = management.create_job(tikeo.api_job("python-echo-api", "demo.echo"))
instance = management.trigger_job(created.id, tikeo.api_trigger())

assert instance.trigger_type == "api"
assert instance.execution_mode == "single"
```

Broadcast requires an explicit helper call. `broadcast_api_trigger` emits `executionMode=broadcast` and a `broadcastSelector`; use it only when all selected workers should receive the API-triggered job.

```python
selector = tikeo.BroadcastSelectorRequest(
    tags=["manual-demo"],
    region="us-east-1",
    labels={"worker_pool": "python-blue"},
)
management.trigger_job(created.id, tikeo.broadcast_api_trigger(selector))
```


## Source-backed reference links

Keep SDK helper docs anchored to source-derived API and protocol references:

- Create helper endpoint: [`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- Trigger helper endpoint: [`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- Instance polling endpoint: [`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- Instance log endpoint: [`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker dispatch message: [`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## Capability discipline

Python is often used to shell out to local tools. Keep that flexibility governed: advertise script runners only when they are installed and controlled, route task logs through task context APIs, and keep SDK diagnostics separate from task execution evidence.

## Evaluation checklist

- Run SDK tests from `sdks/python/tikeo`.
- Run demo dry-run mode to confirm package wiring.
- Start the Server and run live mode when validating Worker Tunnel behavior.
- Confirm Worker visibility, capability snapshots, logs, and task result status.
- Check that missing sandbox tools fail closed instead of being advertised as available.

## Production notes

Use virtual environments or pinned container images for reproducible workers. Keep secret values outside code and pass only references or scoped environment configuration into the worker process.
