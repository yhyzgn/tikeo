# Tikeo SDKs 🧩

[🇨🇳 中文 SDK 文档](../docs/zh-CN/sdk.md)

Tikeo SDKs are language-specific implementations of the same worker and management contracts. The
language may change; the behavior must not.

| Language | Package | Status | What it provides |
| --- | --- | --- | --- |
| Java | `net.tikeo:*` | Release-ready | Worker Tunnel, Spring Boot starters, sandbox tool management, management API client. |
| Rust | `tikeo` | Release-ready | Native Worker Tunnel, script runners, management API client, strict docs/lints. |
| Go | Go module | Release-ready | Worker Tunnel, structured capabilities, sandbox auto tooling, management helpers. |
| Python | `tikeo` | Release-ready | Worker Tunnel, task logs, sandbox runners, management helpers. |
| Node.js | `@yhyzgn/tikeo` | Release-ready | Worker Tunnel, JS/TS-friendly tasks, sandbox runners, management helpers. |

## Shared contract ✅

- Workers connect outbound to the Tikeo Worker Tunnel.
- Dispatch routing uses structured capabilities only.
- Task logs are emitted through task-scoped APIs; SDK diagnostics are separate.
- SDK diagnostics default to `INFO`, write to console, and can also write `tikeo-sdk.log` in a log directory.
- Script execution must run inside a declared sandbox boundary.
- Management clients use app-scoped API keys via `x-tikeo-api-key`.

## Verification

```bash
(cd sdks/java && ./gradlew test --no-daemon)
(cd sdks/rust/tikeo && cargo test --all-features)
(cd sdks/go/tikeo && go test ./...)
(cd sdks/python/tikeo && python -m pytest)
(cd sdks/nodejs/tikeo && bun test && bun run build)
```
