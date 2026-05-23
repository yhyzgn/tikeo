# Java Spring Worker Demo

Runnable Spring Boot demo for `sdks/java/tikee-spring-boot-starter`.

Build and run independently from the repository root:

```bash
(cd examples/java/spring-worker-demo && ./gradlew test)
(cd examples/java/spring-worker-demo && ./gradlew bootRun)
```

The demo defaults to `tikee.worker.dry-run=true` so it can run without a live tikee server and stays running until interrupted. To make it appear in the Worker cluster page, start tikee with `config/dev.toml`, then run:

```bash
(cd examples/java/spring-worker-demo && TIKEE_WORKER_DRY_RUN=false TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 ./gradlew bootRun)
```
