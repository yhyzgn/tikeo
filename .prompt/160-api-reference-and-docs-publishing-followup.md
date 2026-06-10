# 160 — API reference and docs publishing follow-up

## Completed baseline

2026-06-10 completed a repeatable Management API trigger e2e smoke:

- `scripts/management-trigger-e2e-smoke.sh` starts the tikeo server with an isolated SQLite database and generated config under `.dev/reports/management-trigger-e2e-*`.
- The smoke bootstraps admin auth, seeds namespace/app/worker-pool scope, creates a Service Account and SDK API-Key, and verifies the key through `x-tikeo-api-key`.
- It starts the Node.js demo worker in live outbound Worker Tunnel mode (`TIKEO_WORKER_CONNECT=1`) without exposing any worker inbound port.
- It uses the Node.js SDK `ManagementClient` plus `apiJob` / `apiTrigger` to create and trigger a job.
- It asserts `/api/v1/instances/{id}` reaches `succeeded`, `result.success=true`, `result.message=nodejs demo echo processed`, and `/api/v1/instances/{id}/logs` contains worker log evidence.
- `.github/tests/management_smoke_contract_test.py` guards the smoke script contract.
- Main CI `workflow-policy` runs repository contract tests, and `other-cross-language-smoke` runs the Management API trigger smoke after the existing cross-language worker parity smoke and uploads a `management-trigger-e2e-smoke` artifact.

## Verification evidence

- RED: `python3 .github/tests/management_smoke_contract_test.py -k test_management_trigger_smoke_script_is_repeatable_and_source_backed` failed before script creation because the script did not exist.
- RED: `python3 .github/tests/workflow_contract_test.py -k test_cross_language_job_runs_management_trigger_e2e_smoke` failed before CI smoke wiring.
- RED: `python3 .github/tests/workflow_contract_test.py -k test_workflow_policy_runs_repository_contract_tests` failed before repository contract tests were added to `workflow-policy`.
- `python3 .github/tests/management_smoke_contract_test.py` passed.
- `python3 .github/tests/workflow_contract_test.py` passed.
- `python3 .github/tests/docs_site_contract_test.py` passed.
- `TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh` passed; latest local evidence report: `.dev/reports/management-trigger-e2e-20260610T141518Z-129257/management-trigger-e2e-20260610T141518Z-129257.json`.
- `python3 scripts/check-source-size.py` passed.
- `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24` passed.
- `.github/workflows/*.yml` YAML parse passed.
- `git diff --check` passed.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features -- --test-threads=1` passed.
- `cargo build --workspace --all-features` passed.

## Next slice options

1. Add source-derived OpenAPI/protobuf reference pages and link SDK helper docs to exact management endpoints / Worker Tunnel messages.
2. Expand docs user-guide depth from verified UI/backend artifacts:
   - Dashboard
   - Jobs
   - Instances
   - Workers
   - Workflows
   - Scripts
   - Audit
   - Settings
3. Add docs publishing/search/SEO readiness once hosting target is selected:
   - canonical URL
   - robots policy
   - OpenGraph image
   - local search or DocSearch plan
   - maintained `llms.txt` / `llms-full.txt` strategy
4. Optional quality follow-up: add a lighter local-only smoke wrapper or README runbook for `scripts/management-trigger-e2e-smoke.sh` if contributors need a documented one-command flow outside CI.

## Guardrails

- Do not invent SDK helper names in docs; first verify or implement helpers in SDK source.
- Keep SDK management auth app-scoped and machine-to-machine; do not document browser/OIDC sessions as SDK credentials.
- Preserve default API helper behavior: `triggerType=api` and `executionMode=single`.
- Broadcast must remain opt-in through explicit helper/selector APIs and `broadcastSelector` wording.
- Worker demos must connect outbound over Worker Tunnel only; do not add worker inbound ports.
- Keep zh-CN pages complete for every SDK/docs change.
- Keep source files <=1500 lines and rerun `python3 scripts/check-source-size.py` before commit.

## Verification entrypoint

Before committing the next docs/reference slice, run at minimum:

```bash
python3 .github/tests/workflow_contract_test.py
python3 .github/tests/docs_site_contract_test.py
python3 .github/tests/management_smoke_contract_test.py
python3 scripts/check-source-size.py
python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24
git diff --check
cd docs
bun install --frozen-lockfile
bun run docs:typecheck
bun run docs:build
```

For any smoke/e2e follow-up, also run the targeted smoke command and record exact evidence directory/report path.
