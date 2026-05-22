# 036 — Worker Identity Registration Hardening

## Context
Worker clients must not define the authoritative `worker_id`; that caused collision and reconnect ambiguity. The tikee server owns worker identity assignment at Worker Tunnel registration time.

## Current state
- `RegisterWorker` now carries `client_instance_id` as an optional stable client-side hint.
- Server registry generates authoritative ids with `wrk-<uuid-v7>` and returns them in `WorkerRegistered.worker_id`.
- Rust SDK stores the returned worker id and uses it for heartbeat/log/result.
- Java starter properties expose `tikee.worker.client-instance-id`; Java gRPC tunnel implementation still remains future work.

## Next work
1. If implementing Java Worker Tunnel, mirror Rust SDK behavior exactly:
   - send `client_instance_id` only during registration;
   - read `WorkerRegistered.worker_id`;
   - use assigned worker id for heartbeat/log/result.
2. Consider persisting worker registrations when durable worker inventory is introduced, but keep DB foreign-key-free.
3. Keep API/SDK docs explicit: client instance id is metadata, not identity authority.

## Required validation
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features`
- `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`
- `cargo package --manifest-path sdks/rust/tikee/Cargo.toml --allow-dirty`
- Java SDK tests when Gradle distribution is available.
