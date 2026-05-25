# Next Work

## Current priority direction
Continue P1 production governance while preserving the source-size/module-entry rule: every source file must stay <= 1500 lines, and `mod.rs` / `lib.rs` files should remain module entry/re-export surfaces.

## P1 — production hardening / common enterprise use
1. Full script approval/signature/KMS plus URL/File/Secret grants and production release gates.
   - Done foundation: fail-closed policy/signature gates, blocked audit materialization, read-only release-gate preview API, default-disabled local env-secret signature verification for approval tickets, persisted/displayed successful signed release metadata, explicit URL/File/Secret grant request payloads that remain fail-closed, and persisted/displayed verified grant evidence fields for future verifiers.
   - Next: implement a concrete grant verifier boundary (local first, KMS/PKI later), then separately wire Worker-side enforcement before any URL/File/Secret access is enabled.
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
- Verified script releases now persist and display approval ticket, signature, verification time, and verifier identity.
- Script release requests now have explicit `grants.url/file_read/file_write/secret` payloads; non-empty grants fail closed until verified grant enforcement exists.
- Script release pointers can now persist verified grant evidence (`release_grants`) for future verifier paths, but HTTP still does not generate it and Worker access remains disabled.
