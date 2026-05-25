# Next Work

## Current priority direction
Continue P1 production governance while preserving the source-size/module-entry rule: every source file must stay <= 1500 lines, and `mod.rs` / `lib.rs` files should remain module entry/re-export surfaces.

## P1 — production hardening / common enterprise use
1. Continue Go SDK with an ergonomic Worker Tunnel run loop on top of the generated official gRPC bindings.
2. Python SDK is explicitly deferred until later.
3. Node.js SDK is explicitly deferred until later.

## P1 completed
- Script approval/signature/grants production release gate is locally closed: fail-closed policy/signature gates, blocked audit materialization, release-gate preview API, default-disabled env-secret verification, signed release metadata, explicit URL/File/Secret grant request payloads, persisted/displayed grant evidence, and local signed grants bound into the release signature. Worker-side URL/File/Secret access remains disabled until a separate enforcement slice.


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
- Script release pointers can now persist verified grant evidence (`release_grants`); local env-secret signatures can verify and persist signed grants. Worker runtime now carries signed grants, supports container file bind mounts, and fail-closes network/secret grants until safe providers exist.
- OIDC subject-to-local-user tenant scope mapping is closed through governed API/UI and fail-closed callback behavior.
- Prometheus/Grafana recording-rule validation has a committed rules file, Prometheus config, Compose observability profile, Grafana recording-query coverage, and operator runbook.
- Go SDK foundation now uses official `google.golang.org/grpc` / `google.golang.org/protobuf` and generated Worker Tunnel client bindings; Python/Node.js SDKs are deferred.
