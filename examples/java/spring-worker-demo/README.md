# Java Spring Worker Demo

Standard Spring Boot Web demo for `sdks/java/tikee-spring-boot-starter`.

Build, test, and run independently from the repository root. Use the committed Gradle Wrapper; Spring Boot 4 requires Gradle 8.14+ or 9.x, and this demo pins Gradle 9.5.1 for IDE/import consistency.

```bash
(cd examples/java/spring-worker-demo && ./gradlew test)
(cd examples/java/spring-worker-demo && ./gradlew bootRun)
```

The demo test suite covers:

- `EchoProcessorTest` — plain unit test for the example processor behavior.
- `SpringWorkerDemoApplicationTest` — Spring Boot context test for dry-run worker identity, lifecycle startup, capabilities/labels, and `@TikeeProcessor` registry invocation.
- `SpringWorkerDemoDisabledTest` — verifies `tikee.worker.enabled=false` keeps processor discovery but does not create a Worker client.

The demo is a normal embedded-web Spring Boot application. `bootRun` stays online through the web server, not a custom blocking runner. It exposes `GET /demo/health` and `GET /demo/processors` on `TIKEE_DEMO_SERVER_PORT` (default `18080`).

The demo does not configure `client-instance-id`; the SDK creates and reuses a stable local instance id under `~/.tikee/workers` for the configured namespace/app/cluster/region. The demo defaults to `tikee.worker.dry-run=true` so it can run as a local Spring Boot web app without a live tikee server. To make it appear in the Worker cluster page, start tikee with `config/dev.toml`, then run:

```bash
(cd examples/java/spring-worker-demo && TIKEE_WORKER_DRY_RUN=false TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 TIKEE_DEMO_SERVER_PORT=18080 ./gradlew bootRun)
```
