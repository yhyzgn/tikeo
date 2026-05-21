# Java Spring Worker Demo

Runnable Spring Boot demo for `sdks/java/scheduler-spring-boot-starter`.

Build and run independently from the repository root:

```bash
./sdks/java/gradlew -p examples/java/spring-worker-demo test
./sdks/java/gradlew -p examples/java/spring-worker-demo bootRun
```

The current Java SDK client is a safe no-op placeholder until the full Java gRPC Worker Tunnel implementation lands, so this demo validates starter wiring without requiring a live scheduler server.
