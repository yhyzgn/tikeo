# Python Worker demo 🐍

[🇨🇳 中文示例文档](../../../docs/zh-CN/examples.md)

This demo validates Python SDK parity with the Rust, Go, Node.js, and Java workers.

```bash
cd examples/python/worker-demo
python -m pip install -e ../../../sdks/python/tikeo
python -m pip install -e .
TIKEO_WORKER_DRY_RUN=1 python -m tikeo_python_worker_demo
```

Live mode connects to `http://127.0.0.1:9998`, registers in `dev-alpha/orders`, advertises structured
SDK/plugin/script capabilities, and uses SRT/Deno sandbox auto resolution.

Operational cautions: configure SDK diagnostics with `TIKEO_SDK_LOG_LEVEL` and `TIKEO_SDK_LOG_DIR`;
use task context logging for execution evidence.
