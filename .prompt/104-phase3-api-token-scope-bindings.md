# 104 — Phase 3 API token namespace/app/worker-pool scope bindings

## Context
Phase 098-102 completed API token lifecycle, resource/action scopes, TTL, and rotation, but Phase 3 still called out multi-tenant namespace/app/worker-pool scope binding as incomplete.

## Objectives
1. Add API token `scope_bindings` metadata for namespace, app, and worker_pool constraints.
2. Persist bindings without storing plaintext bearer tokens.
3. Expose bindings in token metadata and authenticated principal responses.
4. Enforce namespace/app bindings on job list/create/trigger access for API tokens.
5. Enforce worker_pool bindings on Worker visibility using `worker_pool` / `worker-pool` labels.

## Verification
- RED observed first: `rtk cargo test -p tikeo-server api_token_scope_bindings --all-features` failed because `scope_bindings` were ignored.
- Targeted green: `rtk cargo test -p tikeo-server api_token_scope_bindings --all-features` passed.
- Broader auth green: `rtk cargo test -p tikeo-server api_token --all-features` passed.
- Full gates passed: Rust fmt/clippy/test/build/help, Rust SDK test/wasm/clippy, Web lint/typecheck/test/build, Java Gradle tests.

## Remaining boundaries
- Full tenant/app/worker-pool CRUD and UI management remain future work.
- Real OIDC identity-to-tenant mapping remains open.
