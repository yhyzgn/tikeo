# 039 — Job processor binding model

## Context
Worker Tunnel now has explicit `DispatchTask.processor_name`, and SDKs route on it. The server currently fills it from `job_id` for compatibility because Job definitions do not yet have a dedicated processor binding field.

## Goal
Add first-class processor binding to jobs/workflow nodes so dispatch can target SDK processors independently from job ids.

## Required work
1. Add `processor_name` (or equivalent) to job definitions and workflow job/map nodes, preserving DB no-FK rule.
2. Update migrations/entities/repositories/DTOs/OpenAPI JSON to include processor binding.
3. Update job/workflow create/edit UI forms to configure processor binding.
4. Update dispatcher to fill `DispatchTask.processor_name` from job/node processor binding, falling back to job id only for legacy rows.
5. Add tests for HTTP create/update, dispatcher population, and SDK routing compatibility.

## Validation
- Cargo fmt/clippy/test workspace.
- Web lint/typecheck/test/build if UI changes.
- Rust/Java SDK tests if protocol/SDK behavior changes.
- Update design/.memory/.prompt, commit, push.
