# Java Spring Worker Demo

Standard Spring Boot Web demo for `sdks/java/tikee-spring-boot-starter`.

Build, test, and run independently from the repository root. Use the committed Gradle Wrapper; the demo targets Java 17 bytecode and runs on Spring Boot 3.x to validate the starter against a modern Boot 3 application while the starter itself remains Boot 2.x/3.x compatible.

```bash
(cd examples/java/spring-worker-demo && ./gradlew test)
(cd examples/java/spring-worker-demo && ./gradlew bootRun)
```

The demo test suite covers:

- `EchoProcessorTest` — plain unit test for the example processor behavior.
- `SpringWorkerDemoApplicationTest` — Spring Boot context test for dry-run worker identity, lifecycle startup, capabilities/labels, and `@TikeeProcessor` registry invocation.
- `SpringWorkerDemoDisabledTest` — verifies `tikee.worker.enabled=false` keeps processor discovery but does not create a Worker client.

The demo is a normal embedded-web Spring Boot application. `bootRun` stays online through the web server, not a custom blocking runner. It exposes `GET /demo/health` and `GET /demo/processors` on `TIKEE_DEMO_SERVER_PORT` (default `18080`).

The demo does not configure `client-instance-id`; the SDK creates and reuses a stable local instance id under `~/.tikee/workers` for the configured namespace/app/cluster/region. The demo defaults to `tikee.worker.dry-run=false`, so `bootRun` connects to the live Worker Tunnel at `TIKEE_WORKER_ENDPOINT` and should appear in the Worker cluster page after registration. Start tikee with `config/dev.toml`, then run:

```bash
(cd examples/java/spring-worker-demo && TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 TIKEE_DEMO_SERVER_PORT=18080 ./gradlew bootRun)
```

For local UI-only startup without a tikee server, explicitly set `TIKEE_WORKER_DRY_RUN=true`; dry-run workers do not register with the server and will not appear in the Worker cluster page.

## API-type task management example

In tikee, `scheduleType: api` means the job is created as an explicit API/SDK/UI-triggered task. It does **not** mean the worker executes an HTTP API call. The Java SDK management client can create, enable/disable, and manually trigger these jobs.

When the demo has a management API key, enable the optional control-plane endpoints:

```bash
(cd examples/java/spring-worker-demo &&   TIKEE_MANAGEMENT_ENABLED=true   TIKEE_MANAGEMENT_ENDPOINT=http://127.0.0.1:9999   TIKEE_MANAGEMENT_API_KEY=<tk-api-key>   ./gradlew bootRun)
```

Then call:

- `GET /demo/jobs` — list jobs in the configured namespace/app scope.
- `POST /demo/jobs/echo` — create an `api` schedule job for `demo.echo`, disable/enable it, then trigger it through the SDK.
