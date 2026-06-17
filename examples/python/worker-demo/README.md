# Python Worker demo 🐍

[🇨🇳 中文示例文档](../../../README.zh-CN.md#能证明产品价值的快速开始)

This demo validates Python SDK parity with the Rust, Go, Node.js, and Java workers.

```bash
cd examples/python/worker-demo
python -m pip install -e ../../../sdks/python/tikeo
python -m pip install -e .
TIKEO_WORKER_DRY_RUN=1 python -m tikeo_python_worker_demo
```

Management API create + trigger example:

```bash
TIKEO_API_KEY=<app-scoped-sdk-key> \
TIKEO_HTTP_URL=http://127.0.0.1:9090 \
TIKEO_MANAGEMENT_CREATE_EXAMPLES=1 \
python -m tikeo_python_worker_demo
```

When enabled, the demo creates API-scheduled SDK/plugin jobs and immediately calls
`POST /api/v1/jobs/{job}:trigger`, printing the returned instance id, `triggerType=api`, and `executionMode=single`.

Live mode connects to `http://127.0.0.1:9998`, registers in `dev-alpha/orders`, advertises structured
SDK/plugin/script capabilities, and uses SRT/Deno sandbox auto resolution.

Operational cautions: configure SDK diagnostics with `TIKEO_SDK_LOG_LEVEL` and `TIKEO_SDK_LOG_DIR`;
use task context logging for execution evidence.
