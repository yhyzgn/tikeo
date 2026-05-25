# Next Work

## Current priority direction
Continue P1 production governance while preserving the source-size/module-entry rule: every source file must stay <= 1500 lines, and `mod.rs` / `lib.rs` files should remain module entry/re-export surfaces.

## P1 — production hardening / common enterprise use
1. Full script approval/signature/KMS plus URL/File/Secret grants and production release gates.
   - Done foundation: fail-closed policy/signature gates, blocked audit materialization, read-only release-gate preview API, and default-disabled local env-secret signature verification for approval tickets.
   - Next: persist/display successful signed release metadata and design signed URL/File/Secret grant payloads that remain fail-closed until verified.
2. OIDC tenant/app/role binding and advanced tenant isolation UI.
3. Prometheus/Grafana recording-rule validation and operational runbooks.
4. Go/Python SDKs; Node.js SDK after Worker identity/lifecycle semantics stabilize.

## P2 — ecosystem / advanced differentiation
- PowerJob and XXL-JOB migration tooling.
- Terraform Provider, GitOps/IaC, K8s CRD.
- Task dependency discovery/topology, workflow replay, intelligent scheduling.
- Plugin system, advanced webhook/event sources, task versioning/canary rollback.

## Recently completed
- HTTP/mod.rs and other oversized Rust files were split; max source file line count is 1495.
- Script release-gate preview endpoint was added for local production-gate visibility.
- Script release signatures can be verified locally when `script_governance.release_signature_secret_ref` points at an env secret.
