# 031 — SDK directory follow-up

## Context
- All SDK packages now live under `./sdks`.
- Rust Worker SDK path: `sdks/scheduler-worker-sdk`.
- Java Spring Boot Starter SDK path: `sdks/java`.
- Root Cargo workspace includes `sdks/scheduler-worker-sdk` explicitly.
- Maven validation command is `mvn -f sdks/java/pom.xml -q test`.

## Rules
- Do not add future SDKs under `crates/` or repository root language folders.
- Put future Go/Python/Node SDKs under `sdks/go`, `sdks/python`, `sdks/node` respectively.
- Keep server/backend reusable Rust crates under `crates/`; only SDK-facing packages belong in `sdks/`.
- Update Dockerfile, README, design, memory, and prompts whenever SDK package paths change.
