# 038 — Worker protocol processor key

## Context
Java Spring processor adapter is functional, but it currently routes dispatches by treating `DispatchTask.job_id` as the processor name. That is acceptable for a narrow SDK milestone but should become explicit protocol before broader SDK rollout.

## Current state
- Java SDK real Worker Tunnel client works.
- Spring `@TikeeProcessor` methods are invocable through `SpringTikeeTaskProcessor`.
- Supported signatures: zero args, `TaskContext`, `String`, `byte[]`; supported returns: `TaskOutcome`, `String`, `boolean`, `void`.
- Exceptions map to failed outcomes.

## Goal
Add an explicit processor key/name to Worker Tunnel dispatch semantics and migrate SDKs/server to use it.

## Required work
1. Extend `DispatchTask` proto with `processor_name` or equivalent explicit field while keeping backward compatibility.
2. Update server dispatcher to populate processor key from job definition / workflow node binding.
3. Update Rust SDK `TaskContext` and Java `TaskContext` to expose processor name.
4. Change Java `SpringTikeeTaskProcessor` routing from `jobId()` fallback to explicit processor name, with temporary fallback only for compatibility.
5. Add tests for server proto population and SDK routing.

## Validation
- Cargo fmt/clippy/test workspace.
- Rust SDK test/clippy/package if proto copy changes.
- Java SDK tests and Java demo bootRun.
- Update design/.memory/.prompt, commit, push.
