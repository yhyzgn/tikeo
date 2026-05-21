# 034 — SDK layout normalization, Java Gradle, and examples

## Goal
Normalize SDK and demo layout according to the latest project rule.

## Required target layout
```text
sdks/
├── rust/scheduler-worker-sdk/
├── java/scheduler-java/
├── java/scheduler-spring/
├── java/scheduler-spring-boot-starter/
├── go/scheduler-go-sdk/
├── python/scheduler-python-sdk/
└── nodejs/scheduler-nodejs-sdk/

examples/
├── rust/worker-demo/
├── java/spring-worker-demo/
├── go/worker-demo/
├── python/worker-demo/
└── nodejs/worker-demo/
```

## Rules
- Every SDK must live at `sdks/<language>/<sdk-name>/`, be independently buildable/testable, and be independently publishable with its language-native package manager. No SDK may depend on server-local path modules.
- Every demo must live at `examples/<language>/<demo-name>/` and be independently buildable/runnable.
- Root `Dockerfile` builds only scheduler server; it must not copy/cache/build SDK packages.
- Java SDK uses Gradle, not Maven, with JDK 21+.
- From now on, when SDK/Worker/workflow integration needs end-to-end debugging, autonomously create or update the relevant `examples/<language>/<demo-name>` demo.

## Current status
- Rust SDK is at `sdks/rust/scheduler-worker-sdk`.
- Java SDK is a Gradle multi-project under `sdks/java` with sdk-name subprojects.
- Rust and Java examples are runnable foundations under `examples/rust/worker-demo` and `examples/java/spring-worker-demo`; Go/Python/NodeJS placeholders must become runnable when those SDKs are implemented.

## Next work
1. Implement real Java gRPC Worker Tunnel client and replace the no-op placeholder.
2. When Go/Python/NodeJS SDK work starts, create `sdks/<language>/<sdk-name>` first and immediately convert the matching `examples/<language>/worker-demo` into a runnable demo.
3. Add shard retry / reduce UI work from 033 after SDK layout is stable.

## Hard constraints
- `examples/` is for demos only; runtime config stays in `config/`.
- No database foreign keys.
- HTTP envelope remains `{ code, message, data }`.
- Swagger UI is forbidden.
- After changes run full validation: fmt, clippy, cargo test/build, Java Gradle tests, web checks if docs/API references changed as needed, dev script smoke if runtime changed, update design/.memory/.prompt, commit and push.
