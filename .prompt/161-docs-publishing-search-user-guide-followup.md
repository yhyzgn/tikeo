# 161 — Docs publishing/search/user-guide follow-up

## Completed baseline

2026-06-10 completed source-derived API/protocol reference docs:

- Added `docs/docs/reference/management-openapi.md` and zh-CN mirror.
- Added `docs/docs/reference/worker-tunnel-protobuf.md` and zh-CN mirror.
- Added sidebar entries for both reference pages.
- Extended all Rust, Go, Java Spring Boot, Python, and Node.js SDK docs in English and zh-CN with exact links to:
  - `../reference/management-openapi#post-api-v1-jobs`
  - `../reference/management-openapi#post-api-v1-jobs-job-trigger`
  - `../reference/management-openapi#get-api-v1-instances-instance`
  - `../reference/management-openapi#get-api-v1-instances-instance-logs`
  - `../reference/worker-tunnel-protobuf#dispatchtask`
- Extended `.github/tests/docs_site_contract_test.py` so reference pages remain source-backed and SDK helper docs cannot drift away from exact endpoint/protobuf anchors.
- Recorded the acceptance-stage rigor/context freshness directive in `~/.codex/CONSTITUTION.md`, OMX project memory/notepad, and `.memory/decisions.md`.

## Verification evidence

- RED: `python3 .github/tests/docs_site_contract_test.py DocsSiteContractTest.test_reference_docs_are_source_backed_for_openapi_and_worker_proto DocsSiteContractTest.test_sdk_docs_link_helpers_to_exact_reference_anchors` failed because `docs/docs/reference/management-openapi.md` was missing and SDK docs lacked exact reference links.
- GREEN: the same targeted docs contract passed after adding reference pages and SDK links.
- `python3 .github/tests/workflow_contract_test.py` passed.
- `python3 .github/tests/docs_site_contract_test.py` passed.
- `python3 .github/tests/management_smoke_contract_test.py` passed.
- `python3 scripts/check-source-size.py` passed.
- `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24` passed.
- `.github/workflows/*.yml` YAML parse passed.
- `git diff --check` passed.
- `cd docs && bun install --frozen-lockfile && bun run docs:typecheck && bun run docs:build` passed.
- Docusaurus build output was explicitly checked to contain no `broken anchor` warnings.

## Important notes from the slice

- Bare `{job}` / `{instance}` in MDX link text can become runtime MDX expressions; use code spans around endpoint text.
- Raw `<a id="...">` anchors built but triggered Docusaurus broken-anchor warnings; stable generated heading anchors are preferred for docs links.
- Endpoint anchor text currently comes from headings like `## Post api v1 jobs`, with the exact endpoint preserved directly below as code text. Do not rename these headings without updating docs contracts and SDK links.

## Next slice options

1. Docs publishing/search/SEO readiness:
   - decide or safely default canonical docs URL policy
   - robots policy
   - OpenGraph/social image wiring
   - local search or DocSearch plan
   - maintained `llms.txt` / `llms-full.txt` generation/update strategy
2. User-guide depth from verified UI/backend artifacts:
   - Dashboard
   - Jobs
   - Instances
   - Workers
   - Workflows
   - Scripts
   - Audit
   - Settings
3. Contributor runbook for Management API trigger smoke:
   - document prerequisites
   - document `TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh`
   - explain evidence directory and failure triage
4. Kubernetes controller-specific docs:
   - Nginx / Envoy / Traefik / Gateway API controller values
   - TLS/mTLS certificate mode matrix
   - smoke runbooks

## Guardrails

- Functional/module testing acceptance phase: do not shrink scope; if anything missing/incomplete/untested/hallucinated is found, fill it production-grade or record a real blocker.
- Keep docs source-backed. Do not invent UI features, SDK helpers, endpoints, or protocol messages.
- Keep SDK management auth app-scoped (`x-tikeo-api-key` / `TIKEO_MANAGEMENT_API_KEY`) and machine-to-machine.
- Preserve default API helper behavior: `triggerType=api` and `executionMode=single`; broadcast remains opt-in through explicit helper/selector APIs and `broadcastSelector` wording.
- Worker demos must connect outbound over Worker Tunnel only; do not add worker inbound ports.
- Keep zh-CN pages complete for every docs change.
- Keep source files <=1500 lines and rerun `python3 scripts/check-source-size.py` before commit.
- Use `bun` / `bunx` for docs and frontend commands.

## Verification entrypoint

Before committing the next docs slice, run at minimum:

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

If the slice changes links/anchors, capture build output and verify it has no `broken anchor` warnings.
