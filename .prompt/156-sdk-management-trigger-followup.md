# 156 — SDK management trigger parity follow-up

## Completed baseline

All current SDK families now expose app-scoped Management API helpers that can create API-scheduled jobs and trigger them through `POST /api/v1/jobs/{job}:trigger`:

- Java: already had `TikeoJobClient.triggerJob(...)`, `HttpTikeoJobClient.triggerJob(...)`, `TriggerJobRequest.api()`, and Spring Boot 2/3/4 demo controller endpoints that create and trigger jobs.
- Rust: `ManagementClient::trigger_job(...)`, `ManagementTriggerJobRequest::api()`, `ManagementTriggerJobRequest::broadcast_api(...)`, `ManagementBroadcastSelectorRequest`.
- Go: `ManagementClient.TriggerJob(...)`, `APITrigger()`, `BroadcastAPITrigger(...)`, `BroadcastSelectorRequest`.
- Python: `ManagementClient.trigger_job(...)`, `api_trigger()`, `broadcast_api_trigger(...)`, `BroadcastSelectorRequest`.
- Node.js: `ManagementClient.triggerJob(...)`, `apiTrigger()`, `broadcastApiTrigger(...)`, `BroadcastSelectorRequest`.

Worker demos now show the create+trigger path:

- Rust/Go/Python/Node demos create example SDK/plugin API jobs when `TIKEO_MANAGEMENT_CREATE_EXAMPLES=1`, then trigger each created job and print the returned instance id, `triggerType=api`, and `executionMode=single`.
- Java Spring Boot 2/3/4 demos document `/demo/jobs/echo`, `/demo/jobs/plugin/sql`, and `/demo/jobs/script/{scriptId}` endpoints that create and trigger jobs through the Java SDK.

## Verification evidence

- `go test ./... -count=1` in `sdks/go/tikeo`.
- `bun test && bun run build` in `sdks/nodejs/tikeo`.
- `uv run --extra test python -m pytest` in `sdks/python/tikeo`.
- `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features`.
- `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`.
- `go test ./... -count=1` in `examples/go/worker-demo`.
- `bun test` in `examples/nodejs/worker-demo`.
- `uv run --with '../../../sdks/python/tikeo[test]' --extra test python -m pytest` in `examples/python/worker-demo`.
- `cargo test --manifest-path examples/rust/worker-demo/Cargo.toml`.
- Java Boot2/Boot3/Boot4 demo `./gradlew test --no-daemon`.
- `git diff --check`.
- `python3 scripts/check-source-size.py`.
- `cargo fmt --all -- --check`.

## Next slice options

1. Extend docs site SDK pages with source-backed examples for the new management trigger helpers in all languages.
2. Add an end-to-end management trigger smoke that starts the server, registers a demo worker, creates a job through one SDK, triggers it, and asserts an instance/result transition.
3. Add OpenAPI/protobuf reference pages and link SDK helpers to exact management endpoints.

## Guardrails

- Keep SDK management auth app-scoped through `x-tikeo-api-key`; do not mix it with human session/OIDC token flows.
- Preserve `executionMode=single` as the default API helper behavior and expose broadcast through explicit helpers/selectors.
- Worker demos must keep using real Management API calls; no fake/demo-only trigger paths.
