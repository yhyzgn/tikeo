# 136 — Phase4 next priority after P0 bootstrap

## Context
Phase4 P0 service/operations work is locally closed:
- Worker identity/session lifecycle follows `design/worker-identity-lifecycle-design.md` with Logical Worker / Session / generation / fencing / lease / transport-error evidence and history UI.
- Deployment bootstrap covers Compose, systemd, bare-metal/VM, Worker identity env templates, a systemd Rust worker demo unit, and a readyz + dry-run worker smoke script.

Go SDK run-loop, Python SDK, and Node.js SDK are deferred by user instruction. Helm remains deferred until external DB, secret, gateway, and TLS parameters stabilize.

## Suggested next slice
Pick one non-SDK Phase4 item:
1. PowerJob migration tool/report foundation.
2. XXL-JOB migration tool/report foundation.
3. GitOps/IaC manifest export shape.
4. Task dependency discovery/topology foundation.

## Validation target
- Keep source files <=1500 lines.
- Add focused tests for any parser/exporter/CLI behavior.
- Do not restart Go/Python/Node SDK work unless explicitly requested.
