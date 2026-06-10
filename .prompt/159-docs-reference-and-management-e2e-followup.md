# 159 — Docs reference and Management API e2e follow-up

## Completed baseline

2026-06-10 completed source-backed SDK management create+trigger documentation:

- English and zh-CN SDK docs for Rust, Go, Java Spring Boot, Python, and Node.js now include real helper examples for Management API job create + trigger flows.
- The docs explicitly preserve SDK management auth as app-scoped machine credentials: `x-tikeo-api-key` / `TIKEO_MANAGEMENT_API_KEY`, not OIDC/browser session credentials.
- The docs preserve helper defaults: `triggerType=api`, default `executionMode=single`, and broadcast only through explicit helper/selector APIs that serialize `broadcastSelector`.
- `.github/tests/docs_site_contract_test.py` guards those docs tokens across English and zh-CN SDK pages.
- Java management SDK now has source parity for documented broadcast helper usage: `BroadcastSelectorRequest` and `TriggerJobRequest.broadcastApi(...)`.

## Verification evidence

- RED: `python3 .github/tests/docs_site_contract_test.py -k test_sdk_docs_include_source_backed_management_create_trigger_examples` failed before docs updates because SDK docs lacked `x-tikeo-api-key` / create+trigger coverage.
- RED: `./sdks/java/gradlew -p sdks/java :tikeo:test --tests net.tikeo.management.client.HttpTikeoJobClientTest.supportsExplicitBroadcastApiTriggerSelector --no-daemon` failed before Java implementation because `BroadcastSelectorRequest` did not exist.
- Java targeted helper test passed after implementation.
- Full Java SDK test suite passed: `./sdks/java/gradlew -p sdks/java test --no-daemon`.
- `python3 .github/tests/workflow_contract_test.py` passed.
- `python3 .github/tests/docs_site_contract_test.py` passed.
- `python3 scripts/check-source-size.py` passed.
- `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24` passed.
- `git diff --check` passed.
- `cd docs && bun install --frozen-lockfile && bun run docs:typecheck && bun run docs:build` passed.

## Next slice options

1. Add a repeatable end-to-end Management API trigger smoke:
   - start the server with an isolated temp database
   - create or seed an app-scoped SDK API key/service account
   - register a demo worker
   - create a job through one SDK client
   - trigger it
   - assert instance/result/log transition
2. Add generated/source-derived OpenAPI and protobuf reference pages, then link SDK helper docs to exact management endpoints and Worker Tunnel messages.
3. Expand docs user-guide depth from verified UI/backend artifacts:
   - Dashboard
   - Jobs
   - Instances
   - Workers
   - Workflows
   - Scripts
   - Audit
   - Settings
4. Add docs publishing/search/SEO readiness once hosting target is selected:
   - canonical URL
   - robots policy
   - OpenGraph image
   - local search or DocSearch plan
   - maintained `llms.txt` / `llms-full.txt` strategy

## Guardrails

- Do not invent SDK helper names in docs; first verify or implement helpers in the SDK source.
- Keep SDK management auth app-scoped and machine-to-machine; do not document browser/OIDC sessions as SDK credentials.
- Preserve default API helper behavior: `triggerType=api` and `executionMode=single`.
- Broadcast must remain opt-in through explicit helper/selector APIs and `broadcastSelector` wording.
- Keep zh-CN pages complete for every SDK docs change.
- Keep source files <=1500 lines and rerun `python3 scripts/check-source-size.py` before commit.

## Verification entrypoint

Before committing the next docs/e2e slice, run at minimum:

```bash
python3 .github/tests/workflow_contract_test.py
python3 .github/tests/docs_site_contract_test.py
python3 scripts/check-source-size.py
python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24
./sdks/java/gradlew -p sdks/java test --no-daemon
cd docs
bun install --frozen-lockfile
bun run docs:typecheck
bun run docs:build
```

For e2e smoke work, add the targeted server+worker+SDK smoke command and record the exact database/config isolation used.
