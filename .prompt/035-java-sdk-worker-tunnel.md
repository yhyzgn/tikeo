# 035 — Java SDK Worker Tunnel implementation

## Goal
Replace the Java SDK no-op worker client with a real active outbound gRPC Worker Tunnel client.

## Context
- Java SDK layout: `sdks/java/<sdk-name>/` with Gradle/JDK21+.
- Demo layout: `examples/java/spring-worker-demo/`.
- Root `Dockerfile` is server-only; do not add SDK/demo handling there.
- API envelope, no-FK database rule, and Swagger UI ban remain unchanged.

## Required work
1. Add Java protobuf/gRPC generation or a clean generated-source strategy for `proto/scheduler/worker/v1/worker.proto`.
2. Implement `SchedulerWorkerClient` real connect/register/heartbeat/log/result behavior in `scheduler-java`.
3. Wire Spring Boot properties (`scheduler.worker.endpoint`, identity, labels, capabilities) into the real client.
4. Update `examples/java/spring-worker-demo` so it can smoke-run against either a live scheduler Worker Tunnel or a documented dry-run mode.
5. Add tests for registration config mapping and client lifecycle boundaries.

## Verification
```bash
./sdks/java/gradlew -p sdks/java test
./sdks/java/gradlew -p examples/java/spring-worker-demo test
```

Also run the Rust/backend/web validation set if shared protocol, docs, or server behavior changes.
