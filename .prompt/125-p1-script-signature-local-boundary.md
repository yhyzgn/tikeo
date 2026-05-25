# 125 — P1 script signature local verification boundary

## Context
P1 production governance is active after the P0 service-usability lane. The source-size rule is mandatory: every source file must stay `<=1500` lines, and `mod.rs` / `lib.rs` files should remain module entry/re-export surfaces.

## Completed in this slice
- Added `script_governance.release_signature_secret_ref` config, default disabled.
- Publish/rollback still fail closed when approval/signature metadata is provided and signature verification is not configured.
- When configured with an `env:NAME` secret ref, publish/rollback accepts `approval_ticket` + `signature` only when the signature matches the immutable script id/version/content digest/ticket tuple.
- Release-gate preview reports whether signature verification is configured.

## Next objective
Continue full script production governance without external dependencies by default:
1. Add durable approval/signature metadata/audit visibility for successful signed releases.
2. Design URL/File/Secret grant shape that remains fail-closed until signed approval artifacts explicitly bind grants.
3. Keep Server metadata-only; Workers execute scripts.

## Verification baseline
- `max_source_lines=1495`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help >/tmp/tikee-help.out`
