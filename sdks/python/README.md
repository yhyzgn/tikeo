# Python SDK

Production Python Worker SDK for tikeo. The API mirrors the Rust/Go SDKs: structured worker capabilities, task-scoped logging, management API helpers, and script sandbox runners with Java/Rust/Go-compatible default `auto` resolution (`srt` for native scripts, `deno` for JavaScript/TypeScript).

```bash
cd sdks/python/tikeo
python -m pip install -e .[test]
python -m pytest
```
