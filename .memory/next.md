# Next Work

## Immediate next slice
- Continue with `.prompt/070-phase3-wasm-distribution-integrity-and-gradle10-cleanup.md`.
- Focus areas:
  1. WASM module distribution integrity: digest/signature metadata, worker-side digest validation, and clearer script version binding.
  2. Web/API policy visibility for WASM sandbox settings without enabling unsafe capabilities by default.
  3. Java Gradle 10 compatibility cleanup for deprecated build features reported by Gradle 9.5.1.

## Current status
- Phase 069 completed and verified. Rust SDK can execute WASM dispatch bindings only when feature `wasm` is enabled; Java SDK explicitly rejects unsupported WASM bindings.
- Server Docker remains unrelated to SDK builds; SDKs remain standalone.
