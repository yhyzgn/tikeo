# 133 — P1 Go/Python SDK planning and first implementation slice

## Context
Prometheus/Grafana recording-rule validation is complete. Continue P1 common enterprise adoption by adding non-Java/Rust SDK coverage.

## Goal
Start Go/Python Worker SDK support without destabilizing existing Rust/Java SDKs.

## Recommended first slice
- Inspect current Worker Tunnel proto/package generation patterns.
- Decide whether to start with Go or Python based on build tooling already present in repo.
- Implement one minimal SDK skeleton: registration config, outbound Worker Tunnel connection boundary, heartbeat, and a no-op processor example/test.
- Keep generated/build artifacts out of git unless the repo already commits them for that language.

## Validation target
- Language-specific unit/smoke test for registration/heartbeat shape.
- Existing Rust/Java/server checks remain green for protocol compatibility.
- Source files stay under 1500 lines; module entry files remain re-export only.
