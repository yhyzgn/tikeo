# 150 — Phase 4 production Helm follow-up

## Current context
The Helm chart now has a production baseline: external database Secret injection, conditional SQLite PVC, HTTP/Worker Tunnel TLS/mTLS Secret mounts, generated transport security config, tunable probes/resources/security contexts, server/web ingress, worker identity guidance, and rollback documentation.

## Next recommended slice
1. Check the remote CI for the Helm hardening commit after push.
2. If CI is green, continue with deployment maturity that was intentionally not bundled into the first Helm baseline:
   - optional PodDisruptionBudget templates,
   - NetworkPolicy examples that preserve worker outbound-only semantics,
   - ServiceMonitor / Prometheus scrape examples,
   - Gateway API examples for the Worker Tunnel h2 endpoint,
   - optional chart schema validation (`values.schema.json`).
3. Keep worker Pods/services out of the chart unless there is a separate explicit worker deployment design. Workers still connect outbound only.
4. Do not reintroduce plaintext production database URLs in committed values files.

## Verification entrypoint
- `python3 -m unittest deploy.tests.iac_artifacts_test deploy.tests.smoke_assertions_test`
- `scripts/verify-deploy-bootstrap.sh`
- `helm lint deploy/helm/tikeo`
- `helm template tikeo deploy/helm/tikeo --namespace tikeo`
- `helm template tikeo deploy/helm/tikeo --namespace tikeo -f deploy/helm/tikeo/examples/values-external-postgres.yaml -f deploy/helm/tikeo/examples/values-ingress-tls.yaml`

## Memory/design requirements
Update `.memory/session-log.md`, `.memory/progress.md`, `.memory/next.md`, and deployment sections in `design/tikeo-architecture-design.md` for every completed deployment slice.
