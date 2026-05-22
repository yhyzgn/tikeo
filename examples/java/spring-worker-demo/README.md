# Java Spring Worker Demo

Runnable Spring Boot demo for `sdks/java/tikee-spring-boot-starter`.

Build and run independently from the repository root:

```bash
./sdks/java/gradlew -p examples/java/spring-worker-demo test
./sdks/java/gradlew -p examples/java/spring-worker-demo bootRun
```

The demo defaults to `tikee.worker.dry-run=true` so it can run without a live tikee server. Set `TIKEE_WORKER_DRY_RUN=false` or override `tikee.worker.dry-run=false` plus `tikee.worker.endpoint` to connect the real Java gRPC Worker Tunnel client to a running tikee.
