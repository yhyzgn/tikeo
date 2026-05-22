# 067 — Phase 3 WASM worker runtime executor

## Context
Phase 066 established the stable WASM processor contract in `tikee-core`: `WasmProcessorSpec`, `WasmResourcePolicy`, `WasmCapabilities`, `WasmRuntimeKind`, and validation that denies ambient network/filesystem access by default. Wasmtime 45.x is the selected Rust runtime based on latest crates.io search (`wasmtime = 45.0.0`) and upstream docs for fuel/epoch interruption plus resource limiting.

## Required next work
1. Add a worker-side WASM runtime executor crate/module boundary without moving the backend root entrypoint into `crates/`.
2. Use Wasmtime 45.x only in the worker/runtime boundary, not in server HTTP/storage paths unless needed.
3. Enforce the Phase 066 policy: timeout, fuel, memory cap, no network by default, no preopened directories by default, explicit env allowlist only.
4. Add unit tests around policy-to-runtime configuration and rejection paths. If a tiny WASM fixture is feasible, add a minimal smoke execution test.
5. Preserve API envelope `{ code, message, data }`, no database foreign keys, SDK independence, and no Docker SDK build coupling.
6. Update `design/tikee-architecture-design.md`, `.memory/*`, and create `.prompt/068-*.md`.
7. Run full verification, commit with Lore-style trailers, and push.
