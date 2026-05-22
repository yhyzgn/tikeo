# Next Work

## Immediate next slice
- Continue with `.prompt/077-script-execution-governance-after-tikee-rename.md`.
- Focus areas:
  1. Add script-bound execution governance visibility: audit/result classification for missing worker capability, missing runner, policy rejection, digest mismatch, timeout, runtime unavailable, and output-limit failures.
  2. Add an optional live smoke path for containerized script runner execution when Docker/compatible runtime is available, while keeping CI/unit tests deterministic without Docker.
  3. Keep Server as metadata dispatcher only; all script execution remains Worker-side and opt-in.

## Current status
- Project identity has been renamed from the previous project identity to tikee.
- Rust binary/crates, Docker/Compose/K8s naming, proto package, Rust SDK crate/path, Java Gradle modules, Java package prefix (`com.yhyzgn.tikee`), docs, memory, and prompts now use tikee naming.
- Phase 075 functionality remains the current implementation baseline: Rust SDK includes opt-in `ContainerScriptRunner` for non-WASM scripts with default-deny Docker-compatible command construction.

## SDK naming note
- Rust SDK is `sdks/rust/tikee` / crate `tikee`. Java core SDK module/artifact is `tikee`; Java package prefix remains `com.yhyzgn.tikee`.
