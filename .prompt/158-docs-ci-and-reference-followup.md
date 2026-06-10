# 158 — Docs CI and reference-depth follow-up

## Completed baseline

2026-06-10 completed docs CI verification wiring:

- Main CI includes a dedicated `Docs site` job after `workflow-policy`.
- The job runs:
  - `python3 .github/tests/docs_site_contract_test.py`
  - `cd website && bun install --frozen-lockfile`
  - `bun run docs:typecheck`
  - `bun run docs:build`
- `website/bun.lock` tarball URLs now use public `https://registry.npmjs.org/` instead of the previous private Nexus registry host.
- `.github/tests/workflow_contract_test.py` guards the `Docs site` CI job shape.
- `.github/tests/docs_site_contract_test.py` guards that the docs lockfile does not depend on private registry tarball URLs.

## Verification evidence

- RED: `python3 .github/tests/workflow_contract_test.py -k test_ci_runs_docs_site_verification` failed before the CI job existed with `job not found: docs-site`.
- RED: `python3 .github/tests/docs_site_contract_test.py -k test_docs_lockfile_uses_public_registry_for_ci` failed before the lockfile registry normalization because `website/bun.lock` contained `nexus3.recycloud.cn`.
- `python3 .github/tests/workflow_contract_test.py` passed.
- `python3 .github/tests/docs_site_contract_test.py` passed.
- `python3 scripts/check-source-size.py` passed.
- `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24` passed.
- `.github/workflows/*.yml` YAML parse passed.
- `cd website && bun install --frozen-lockfile && bun run docs:typecheck && bun run docs:build` passed.
- `git diff --check` passed.

## Next slice options

1. Extend docs site SDK pages with source-backed examples for all-language Management API create+trigger helpers:
   - `triggerType=api`
   - default `executionMode=single`
   - explicit broadcast selector helpers
   - app-scoped `x-tikeo-api-key` / SDK Management API-Key behavior
2. Add a repeatable end-to-end management trigger smoke:
   - start the server
   - register a demo worker
   - create a job through one SDK
   - trigger it
   - assert instance/result transition
3. Add docs search/publish readiness once hosting target is selected:
   - canonical URL
   - robots policy
   - OpenGraph image
   - local search or DocSearch plan
   - generated/maintained `llms.txt` strategy
4. Expand user/reference depth from verified artifacts:
   - Dashboard, Jobs, Instances, Workers, Workflows, Scripts, Audit, Settings user guides
   - source-derived OpenAPI/protobuf references
   - configuration/environment variable matrix generated from committed config structures or examples

## Guardrails

- Do not remove the `Docs site` CI job unless a replacement docs-specific workflow provides equivalent checks.
- Do not let `website/bun.lock` regress to private registry tarball URLs.
- Keep zh-CN pages complete for every new P0 sidebar route.
- Do not invent API schemas manually when OpenAPI/protobuf generation can own the reference.
- Keep SDK management auth app-scoped; do not mix SDK API-Key docs with human OIDC/session token flows.
- Preserve `executionMode=single` as the default API helper behavior and expose broadcast through explicit helpers/selectors.

## Verification entrypoint

Before committing the next docs slice, run:

```bash
python3 .github/tests/workflow_contract_test.py
python3 .github/tests/docs_site_contract_test.py
python3 scripts/check-source-size.py
python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24
cd website
bun install --frozen-lockfile
bun run docs:typecheck
bun run docs:build
```

For route/baseUrl changes, also run `bun run docs:serve` and curl affected English and zh-CN routes under both `/` and `/tikeo/` when relevant.
