# 151 — Phase 4 Helm ops maturity follow-up

## Current context
The Helm chart now includes the production baseline plus optional operations hardening:
- external database Secret injection,
- conditional SQLite PVC,
- HTTP and Worker Tunnel TLS/mTLS Secret wiring,
- server/web ingress,
- probes/resources/security knobs,
- PodDisruptionBudget,
- NetworkPolicy preserving Worker outbound-only semantics,
- ServiceMonitor for `/metrics`,
- Gateway API `GRPCRoute` example for Worker Tunnel h2/gRPC,
- `values.schema.json` validation.

## Next recommended slice
1. Check remote CI/Coverage for the Helm ops maturity commit after push.
2. If green, choose one of these next safe branches:
   - source-size debt cleanup or explicit CI audit boundaries for historical >1500-line files,
   - docs site implementation from `design/docs-site-build-plan.md` if the user wants to start the actual site,
   - Kubernetes deployment docs refinement around real ingress-controller examples for Nginx/Envoy/Traefik.
3. Do not add business Worker Deployments or worker inbound Services to the server/web Helm chart.
4. Keep committed values free of production credentials.

## Verification entrypoint
- `python3 -m unittest deploy.tests.iac_artifacts_test deploy.tests.smoke_assertions_test`
- `scripts/verify-deploy-bootstrap.sh`
- `helm lint deploy/helm/tikeo` with default and example values
- `helm template` for default, external DB, TLS, and ops/Gateway overlays
- normal repo fmt/clippy/test/build checks before commit
