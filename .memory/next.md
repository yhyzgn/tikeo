# Next Work

## Current priority direction
Rebalance remaining Phase 3 / Phase 4 work around service usability first. Prefer items that make tikee safer and easier to run as a real shared service before ecosystem or migration features.

## P0 — service usage / production blockers
1. Worker identity/session lifecycle governance for K8s/Docker and bare metal/VM/systemd: logical worker, session generation, fencing token, lost-reason evidence, history UI.
   - Done: in-memory logical key, generation, heartbeat fencing, replacement reason, latest-online worker list, Rust/Java heartbeat token echo.
   - Remaining next: persistent logical/session/event tables, lease scanner lost reasons, graceful unregister, assignment token validation, Web layered UI/history.
2. Deployment/operations bootstrap: Compose/systemd/bare-metal templates first; Helm after production parameters for external DB, secrets, gateway, and TLS settle.
3. Production alert delivery hardening: SMTP TLS/auth/secret references, provider secrets, retry/DLQ visibility, minimal live smoke.

## P1 — production hardening / common enterprise use
- Full script approval/signature/KMS plus URL/File/Secret grants and production release gates.
- OIDC tenant/app/role binding and advanced tenant isolation UI.
- Prometheus/Grafana recording-rule validation and operational runbooks.
- Go/Python SDKs; Node.js SDK after Worker identity/lifecycle semantics stabilize.

## P2 — ecosystem / advanced differentiation
- PowerJob and XXL-JOB migration tooling.
- Terraform Provider, GitOps/IaC, K8s CRD.
- Task dependency discovery/topology, workflow replay, intelligent scheduling.
- Plugin system, advanced webhook/event sources, task versioning/canary rollback.

## Deferred boundary reminders
- Node.js SDK, K8s Helm, PowerJob migration, and XXL-JOB migration remain Phase 4, but Helm/deployment bootstrap should be prioritized by service usability once core production parameters are stable.

## Recently completed
- P0 OIDC external subject -> local user/role/tenant mapping and opaque tikee session issuance is complete; local login state remains opaque `auth_sessions` bearer tokens, not JWT.
- P0 real HTTP and Worker Tunnel TLS/mTLS listeners are complete; HTTP reloads certificate material per new TLS connection, Worker Tunnel starts with tonic TLS/mTLS, and diagnostics report `tls|mtls|tls_config_error`.
