# Tikeo Python Worker SDK 🐍

[🇨🇳 中文 SDK 文档](../../../README.zh-CN.md#行为一致的-sdk)

Python SDK aligned with the Java, Rust, Go, and Node.js Worker SDKs.

## Runtime requirements

- Python 3.11+ is required by `pyproject.toml`.
- CI verifies the SDK and demo with Python 3.12.

## Features

- Worker Tunnel client with structured capabilities.
- Task processors and precise task-scoped logs.
- Standard-library SDK diagnostics through `configure_logging(LogConfig.from_env())`.
- Optional SDK file output to `tikeo-sdk.log`.
- Management API client using `x-tikeo-api-key`.
- SRT/Deno/container/local script runners and fail-closed unavailable handlers.

## Usage

```python
from tikeo import Client, LogConfig, configure_logging, local_config

configure_logging(LogConfig.from_env())
config = local_config("http://127.0.0.1:9998", "orders-python-1")
config.namespace = "dev-alpha"
config.app = "orders"
config.add_normal_processor("demo.echo", "Echo payload demo processor")
client = Client(config)
```

## Operational cautions

- Sandbox auto-install is background prewarm only: SDK startup never waits for downloads; missing tools stay unadvertised and fail closed until available.
- Set `TIKEO_SANDBOX_STRICT_ISOLATION=1` when strict sandbox isolation is required; this skips host `PATH` tools/interpreters and uses only sandbox-tools cache binaries.
- Do not log API keys or raw payloads through SDK diagnostics.
- Use task context logging for execution output that belongs in instance logs.
- Keep script runners fail-closed when sandbox tools are unavailable.

## Verification

```bash
python -m pip install -e .[test]
python -m pytest
```
