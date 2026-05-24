# Phase 123 / P0 real TLS/mTLS listeners

## Goal
Replace the TLS pending-listener boundary with real HTTP and Worker Tunnel TLS/mTLS listener wiring and actionable startup/status diagnostics.

## Implementation
- HTTP listener can serve real HTTPS via rustls. The listener validates TLS material at startup and reloads certificate/key/CA files for each new connection so rotation is picked up without process restart.
- Worker Tunnel listener accepts tonic `ServerTlsConfig` built from the same endpoint configuration, including optional mTLS client CA verification.
- Transport diagnostics no longer report `tls_pending_listener`; modes are `plaintext`, `tls`, `mtls`, and `tls_config_error`.
- TLS dependencies are explicit in `crates/tikee-server/Cargo.toml`; cert/key/CA parsing lives in `crates/tikee-server/src/transport_security.rs`.

## Verification
- `rtk cargo test -p tikee-server http_tls_listener_serves_https_when_configured --all-features`
- `rtk cargo test -p tikee-server transport_security_status_reports_defaults_and_partial_mtls_config --all-features`
- `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikee-help.out'`
