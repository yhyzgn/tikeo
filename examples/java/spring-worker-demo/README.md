# Java Spring Worker Demo

Runnable Spring Boot demo for `sdks/java/scheduler-spring-boot`.

Build and run independently from the repository root:

```bash
./sdks/java/gradlew -p examples/java/spring-worker-demo test
./sdks/java/gradlew -p examples/java/spring-worker-demo bootRun
```

The demo defaults to `scheduler.worker.dry-run=true` so it can run without a live scheduler server. Set `SCHEDULER_WORKER_DRY_RUN=false` or override `scheduler.worker.dry-run=false` plus `scheduler.worker.endpoint` to connect the real Java gRPC Worker Tunnel client to a running scheduler.
