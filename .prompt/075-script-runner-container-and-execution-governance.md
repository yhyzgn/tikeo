# 075 — Containerized script runner and execution governance

## Context
Phase 074 completed non-WASM script protocol binding. The Server now binds approved released immutable `script_versions` snapshots into Worker Tunnel `ScriptProcessorBinding`, filters workers by `script:<language>` / `script:*` / `*`, and never executes user code. Rust SDK execution is explicit opt-in through `ScriptRunnerRegistry`; Java SDK rejects script bindings for now.

## Objectives
1. Add a safer Worker-side containerized runner option for non-WASM scripts as the next boundary after local subprocess execution.
2. Preserve explicit opt-in: no worker executes scripts unless it registers the runner and advertises the matching capability.
3. Keep default-deny policy enforcement for network, filesystem, secrets, env, timeout, memory, and output caps.
4. Add execution governance visibility for script-bound tasks: clear result messages, audit-friendly fields, and Web/API hints for missing runner/capability/policy rejection.
5. Document operational requirements for Docker/K8s runner deployment without requiring tikee Server to have Docker access.

## Constraints
- Server must never execute user code and must not require Docker/K8s privileges for script execution.
- Dispatch must use released immutable `script_versions` snapshots only; draft/current script content is not executable.
- DB relationships remain soft-only; do not add foreign keys.
- API responses remain `{ code, message, data }`.
- SDKs remain independently buildable/publishable.
- Do not add Swagger.

## Suggested implementation notes
- Prefer a runner abstraction that can support both Docker CLI/local daemon and future Kubernetes Job/Pod runner without changing Worker Tunnel protocol.
- If Docker/K8s execution cannot be fully implemented safely in one slice, implement a bounded interface + deterministic tests and keep actual runner opt-in behind feature/config flags.
- Avoid leaking host env vars or mounting host paths by default.
- Keep network disabled by default; only future signed grants should enable egress.

## Expected verification
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -- --help`
- `cd web && bun run typecheck && bun test && bun run build`
- `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`
- `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`
- `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`
- `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`.
- Create the next `.prompt/076-*.md` before commit.
- Mark completed roadmap items in `design/` using `[x]` only, no ✅.
- Commit with Lore trailers and push.
