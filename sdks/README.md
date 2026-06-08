# Tikeo SDKs 🧩

[🇨🇳 中文 SDK 文档](../README.zh-CN.md#行为一致的-sdk)

Tikeo SDKs are language-specific implementations of the same worker and management contracts. The
language may change; the behavior must not.

| Language | Package | Runtime requirement | Status | What it provides |
| --- | --- | --- | --- | --- |
| Java | `net.tikeo:*` | Java 17+; CI verifies with Temurin 21. | Release-ready | Worker Tunnel, Spring Boot starters, sandbox tool management, management API client. |
| Rust | `tikeo` | Rust 1.95+. | Release-ready | Native Worker Tunnel, script runners, management API client, strict docs/lints. |
| Go | Go module | Go 1.26+. | Release-ready | Worker Tunnel, structured capabilities, sandbox auto tooling, management helpers. |
| Python | `tikeo` | Python 3.11+; CI verifies with Python 3.12. | Release-ready | Worker Tunnel, task logs, sandbox runners, management helpers. |
| Node.js | `@yhyzgn/tikeo` | Node.js 24+; Bun for repository scripts. | Release-ready | Worker Tunnel, JS/TS-friendly tasks, sandbox runners, management helpers. |

## Runtime requirements

- Java SDKs: Java 17+; CI verifies with Temurin 21.
- Rust SDK: Rust 1.95+.
- Go SDK: Go 1.26+.
- Python SDK: Python 3.11+; CI verifies with Python 3.12.
- Node.js SDK: Node.js 24+; Bun is used for repository test/build scripts.

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
