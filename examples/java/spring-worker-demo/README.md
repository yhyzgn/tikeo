# Java Spring Worker Demo

Runnable Spring Boot demo for `sdks/java/tikee-spring-boot-starter`.

Build, test, and run independently from the repository root. Use the committed Gradle Wrapper; Spring Boot 4 requires Gradle 8.14+ or 9.x, and this demo pins Gradle 9.5.1 for IDE/import consistency.

```bash
(cd examples/java/spring-worker-demo && ./gradlew test)
(cd examples/java/spring-worker-demo && ./gradlew bootRun)
```

The demo test suite covers:

- `EchoProcessorTest` — plain unit test for the example processor behavior.
- `SpringWorkerDemoApplicationTest` — Spring Boot context test for dry-run worker identity, lifecycle startup, capabilities/labels, and `@TikeeProcessor` registry invocation.
- `SpringWorkerDemoDisabledTest` — verifies `tikee.worker.enabled=false` keeps processor discovery but does not create a Worker client.

Tests set `tikee.worker.demo.block-on-startup=false`; normal `bootRun` keeps blocking so the demo remains online for server + Worker UI联调.

The demo defaults to `tikee.worker.dry-run=true` so it can run without a live tikee server and stays running until interrupted. To make it appear in the Worker cluster page, start tikee with `config/dev.toml`, then run:

```bash
(cd examples/java/spring-worker-demo && TIKEE_WORKER_DRY_RUN=false TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 ./gradlew bootRun)
```
