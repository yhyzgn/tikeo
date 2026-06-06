# 121 — Phase 3 / Phase 4 service-usage priority rebalance

## Goal
Rebalance the remaining Phase 3 and Phase 4 roadmap so features that directly affect service adoption, secure production use, day-2 operations, and daily Worker usability are implemented before ecosystem/nice-to-have capabilities.

## Priority rule
- **P0 Service usability / production blockers**: without this, a real team cannot safely run or operate tikeo as a shared service.
- **P1 Production hardening / common enterprise use**: important for broader rollout, but the core service can run while these are completed.
- **P2 Ecosystem / advanced differentiation**: useful, but not required for the first stable production adoption path.

## Rebalanced P0 candidates
1. OIDC external subject mapping to local user/role/tenant with opaque session issuance.
2. Real HTTP and Worker Tunnel TLS/mTLS listeners plus certificate reload/rotation.
3. Worker identity/session lifecycle governance across K8s/Docker and bare metal/VM/systemd.
4. Deployment packaging and operational bootstrap: Compose/systemd first, Helm next, with external DB and secret wiring.
5. Production alert delivery hardening for SMTP/provider auth/secret handling and reliable retry visibility.

## Rebalanced P1 candidates
1. Full script approval/signature/KMS and URL/File/Secret grants for production script release.
2. OIDC tenant binding and advanced tenant isolation policy UI.
3. Prometheus/Grafana recording-rule validation and operational runbooks.
4. Go/Python SDKs for common non-Java/Rust adoption.
5. Node.js SDK after worker lifecycle and SDK identity semantics stabilize.

## Rebalanced P2 candidates
1. PowerJob and XXL-JOB migration tooling.
2. Terraform/GitOps/CRD ecosystem integrations.
3. Task topology discovery, intelligent scheduling, workflow replay.
4. Plugin system, advanced webhook/event sources, job canary/version rollback.

## Out of scope for this prompt
Implementation of the items above. This prompt only records the roadmap ordering change.
