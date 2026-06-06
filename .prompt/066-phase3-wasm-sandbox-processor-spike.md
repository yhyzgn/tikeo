# 066 — Phase 3 WASM sandbox processor spike

## Context
Phase 3 Web governance is complete through route metadata, lazy loading, unified 401/403 handling, and URL query persistence. The roadmap now moves toward secure dynamic execution.

## Required next work
1. Design the WASM sandbox processor boundary before broad dynamic language execution.
2. Evaluate a Rust WASM runtime crate suitable for tikeo worker-side execution with resource limits, timeout, memory cap, env/input isolation, and no ambient network/filesystem by default.
3. Add a minimal backend/core processor model extension only if needed; keep server entrypoint outside `crates/` and keep SDKs independently publishable.
4. Preserve API envelope `{ code, message, data }` and the no-foreign-key database rule.
5. Update `design/tikeo-architecture-design.md`, `.memory/*`, and create `.prompt/067-*.md`.
6. Run verification appropriate to changed code (`cargo fmt/clippy/test`, web checks if touched), commit with Lore-style trailers, and push.

## Notes
- Go SDK and Python SDK remain Phase4.
- Docker/server image must not include SDK build concerns.
- Dynamic script security must prefer sandbox/container boundaries over host execution shortcuts.
