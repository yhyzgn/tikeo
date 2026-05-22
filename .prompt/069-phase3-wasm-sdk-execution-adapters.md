# 069 — Phase 3 WASM SDK execution adapters

## Context
Phase 068 added dispatch metadata for approved `language=wasm` scripts: `DispatchTask.processor_binding` with `WasmProcessorBinding`. Server only passes approved script module bytes and policy metadata to workers; it does not execute user code. Rust `scheduler-wasm` already provides the Wasmtime executor boundary.

## Required next work
1. Update Rust Worker SDK to recognize `processor_binding.wasm` and route it through `scheduler-wasm` when enabled, while preserving normal `TaskProcessor` behavior for regular SDK processors.
2. Decide Java SDK behavior for WASM binding: either explicit unsupported result with clear message or a documented future adapter; do not silently ignore dynamic bindings.
3. Add tests in Rust SDK for WASM binding execution / rejection paths. Add Java tests if touching Java SDK.
4. Keep SDK packages independently buildable/publishable; do not couple server Dockerfile to SDK builds.
5. Preserve API envelope, no DB foreign keys, and Server-does-not-execute-user-code invariant.
6. Update design/.memory and create `.prompt/070-*.md`.
7. Run full verification, commit with Lore-style trailers, and push.

## Notes
- Java Gradle test may require cached Gradle distribution; if first download is slow, document the verification gap rather than blocking unrelated Rust/server work indefinitely.
