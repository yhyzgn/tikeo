# tikee Python Worker SDK

Python SDK aligned with the Rust, Go, and Java Worker SDKs.

Highlights:

- Worker Tunnel client with structured capabilities.
- Task processors with precise task-scoped logs.
- Management API client using `x-tikee-api-key`.
- Script runners for SRT, Deno, container, local development, and fail-closed unavailable handlers.
- Default script sandbox resolution: `srt` for shell/Python/PowerShell/PHP/Groovy/Rhai, `deno` for JavaScript/TypeScript.

```bash
python -m pip install -e .[test]
python -m pytest
```
