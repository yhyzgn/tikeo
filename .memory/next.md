# Next Work

## Current priority direction
Server + Web + Java SDK/Demo 联合自动化测试清单已复核为全绿；下一步若继续测试增强，优先把当前 DOM/JSON evidence 升级为真实浏览器 screenshot/video CI 产物。功能开发方向继续保持 Phase4 P2 advanced differentiation，同时保留单文件 <=1500 行和 `mod.rs` / `lib.rs` 只做入口/re-export 的约束。

## Phase4 P0 — service usage / operations first
1. Worker identity/session lifecycle governance is now aligned to `design/worker-identity-lifecycle-design.md`.
2. Deployment and operations bootstrap is locally closed for Compose/systemd/bare-metal; next Phase4 P0 gap is production packaging/Helm only after external DB/secret/gateway/TLS params stabilize.
3. Go SDK run-loop, Python SDK, and Node.js SDK are explicitly deferred until later.

## P1 completed
- Script approval/signature/grants production release gate is locally closed: fail-closed policy/signature gates, blocked audit materialization, release-gate preview API, default-disabled env-secret verification, signed release metadata, explicit URL/File/Secret grant request payloads, persisted/displayed grant evidence, and local signed grants bound into the release signature. Worker-side URL/File/Secret access remains disabled until a separate enforcement slice.


## P2 — ecosystem / advanced differentiation
- PowerJob and XXL-JOB migration tooling.
- GitOps/IaC manifest export/diff、Terraform provider live drift smoke、K8s CRD schema 和 operator dry-run 均已在联合自动化测试状态表中验证通过；后续可继续做真实集群/真实浏览器 CI 增强。
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
- Go SDK foundation now uses official `google.golang.org/grpc` / `google.golang.org/protobuf` and generated Worker Tunnel client bindings; Go run-loop/Python/Node.js SDKs are deferred.
- Worker identity/session lifecycle now follows `design/worker-identity-lifecycle-design.md`, including transport-error evidence when gRPC streams end without graceful unregister.
- Deployment bootstrap now includes Compose/systemd/bare-metal docs, Worker identity env templates, a systemd Rust worker demo unit, and a readyz/worker dry-run smoke script.

## Recently completed — 2026-05-31
- SDK API-Key Service Account lifecycle is upgraded to the long-term model: Service Account is independently managed, API-Key creation selects existing active identity, disable revokes bound keys, and Web/API/smoke tests follow the new flow.

## Recently completed — 2026-06-02
- 联合自动化测试方案与可执行状态计划已重新核对并同步：所有测试项为通过/已配置/已沉淀，无测试项级待执行、失败或阻塞残留。
- 当前测试验收总览：80/80 通过（P0-A/P0-B/P0-C/P0-D/P1-E/P1-F/P2-G/数据库专项）。

## Java SDK starter compatibility
- Use `tikee-spring-boot-starter` for Spring Boot 4.x.
- Use `tikee-spring-boot2-starter` for Spring Boot 2.x.
- Use `tikee-spring-boot3-starter` for Spring Boot 3.x.
- Keep Java source and tests compatible with Java 17 APIs while `--release 17` is configured.
