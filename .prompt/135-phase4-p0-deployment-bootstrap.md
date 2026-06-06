# 135 — Phase4 P0 deployment and operations bootstrap

## Context
Phase4 P0-1 Worker identity/session lifecycle governance is now aligned to `design/worker-identity-lifecycle-design.md`: Logical Worker / Session / generation / fencing token, persistent sessions/events, graceful unregister, replacement fencing, assignment-token validation, lease timeout scanner, transport-error evidence, and Worker lifecycle history UI are in place.

Go SDK remaining run-loop work is intentionally deferred to be handled later together with Python SDK planning. Node.js SDK is also deferred by user instruction.

## Next slice
Continue Phase4 P0 with deployment/operations bootstrap:
- Compose/systemd/bare-metal worker/server templates first.
- Document stable `client_instance_id` recommendations for K8s, Docker, systemd, VM/bare metal, and local dev.
- Provide environment variable examples for `TIKEO_WORKER_HOST_ID`, service name, instance slot, namespace/app/cluster/region, and worker pool label.
- Add smokeable examples that do not require exposing inbound business ports.

## Validation target
- Rust format/clippy/tests for touched crates.
- Web tests if UI/docs surface changes.
- Source-size scan remains <=1500 lines per source file.
