# 068 — Phase 3 WASM script binding and dispatch integration

## Context
Phase 067 added a dedicated `scheduler-wasm` crate as the worker-side Wasmtime executor boundary. It enforces `WasmProcessorSpec` with fuel, timeout/epoch interruption, memory cap, no ambient WASI imports, and default-deny network/filesystem capabilities. Server HTTP/storage remain decoupled from the Wasmtime dependency.

## Required next work
1. Design how approved `language=wasm` scripts bind to processor names and worker dispatch without making the server execute user code.
2. Add metadata/DTO mapping only if needed so workers can discover or receive WASM module bytes/spec safely; preserve API envelope `{ code, message, data }`.
3. Keep database relationships soft-only; do not add foreign keys.
4. Add tests for WASM script policy validation, approved-only execution eligibility, and dispatch metadata shape.
5. Update Web/UI only if necessary to expose WASM sandbox policy clearly.
6. Update `design/scheduler-architecture-design.md`, `.memory/*`, and create `.prompt/069-*.md`.
7. Run full verification, commit with Lore-style trailers, and push.
