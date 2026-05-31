# Phase 4 Service Account / SDK API-Key hardening follow-up

Current baseline:
- Service Account is a first-class app-scoped machine identity.
- API-Key creation must select an existing active Service Account by `service_account_id`; implicit `service_account_name` creation is no longer allowed.
- Disabling a Service Account revokes bound active API-Keys; API-Key authentication checks the bound Service Account still exists and is active.
- Web `/api-keys` manages Service Accounts and API-Key credentials in one operational page.

Validation anchors from the closing slice:
- `cargo check -p tikee-server`
- `cargo test -p tikee-server sdk_api_key -- --nocapture`
- `cargo test -p tikee-server disabling_service_account_revokes_bound_sdk_keys -- --nocapture`
- `cd web && bun run typecheck`
- `cd web && bun test --run client.test.ts`

Next hardening ideas:
1. Add live smoke coverage that disables a Service Account and verifies the old `X-Tikee-API-Key` fails against a running server.
2. Add Web page interaction/e2e coverage for Service Account create/edit/disable when browser automation is enabled.
3. Consider splitting storage compatibility helpers if `crates/tikee-storage/src/lib.rs` grows further; do not add clippy allow attributes for file size/too-many-lines.
