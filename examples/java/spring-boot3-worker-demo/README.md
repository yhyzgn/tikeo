# Java Spring Boot 3.x Worker Demo

Standard Spring Boot Web demo for `sdks/java/tikeo-spring-boot3-starter`, validating the compatibility starter in a Spring Boot 3.x application.

Build, test, and run independently from the repository root. Use the committed Gradle Wrapper; the demo targets Java 17 bytecode and runs on Spring Boot 3.x to validate the matching tikeo starter artifact.

```bash
(cd examples/java/spring-boot3-worker-demo && ./gradlew test)
(cd examples/java/spring-boot3-worker-demo && ./gradlew bootRun)
```

The demo test suite covers:

- `EchoProcessorTest` — plain unit test for the example processor behavior.
- `SpringWorkerDemoApplicationTest` — Spring Boot context test for dry-run worker identity, lifecycle startup, capabilities/labels, and `@TikeoProcessor` registry invocation.
- `SpringWorkerDemoDisabledTest` — verifies `tikeo.worker.enabled=false` keeps processor discovery but does not create a Worker client.
- `SpringBootStarterCompatibilityMatrixTest` — verifies this demo uses `tikeo-spring-boot3-starter` and that the Java SDK exposes real Boot 2, Boot 3, and Boot 4 starter modules with concrete `src` trees.

The demo is a normal embedded-web Spring Boot application. `bootRun` stays online through the web server, not a custom blocking runner. It exposes `GET /demo/health` and `GET /demo/processors` on `TIKEO_DEMO_SERVER_PORT` (default `18083`).

The demo configures `client-instance-id` to `spring-boot3-worker-demo` by default so multiple Java demos in the same namespace/app still register as separate workers. Override `TIKEO_WORKER_CLIENT_INSTANCE_ID` only when intentionally testing worker identity reuse. The demo defaults to `tikeo.worker.dry-run=false`, so `bootRun` connects to the live Worker Tunnel at `TIKEO_WORKER_ENDPOINT` and should appear in the Worker cluster page after registration. Start tikeo with `config/dev.toml`, then run:

```bash
(cd examples/java/spring-boot3-worker-demo && TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998 TIKEO_DEMO_SERVER_PORT=18083 ./gradlew bootRun)
```


For the manual acceptance defaults, prefer the wrapper script because it sets the demo scope and management scope consistently:

```bash
(cd examples/java/spring-boot3-worker-demo && ./scripts/run-demo-worker.sh)
```

For local UI-only startup without a tikeo server, explicitly set `TIKEO_WORKER_DRY_RUN=true`; dry-run workers do not register with the server and will not appear in the Worker cluster page.


## Manual integration acceptance scope

By default `scripts/run-demo-worker.sh` starts this demo in the same scope used by `scripts/dev-integration-seed.sh` and `scripts/start-java-demo-workers.sh`:

- scope: `dev-alpha/orders/boot3-blue`
- advertised processors: `demo.echo, demo.context, demo.bytes, demo.heartbeat, demo.report, demo.workflow.step, demo.fail, sql:billing.sql-sync`
- health check: `GET /demo/health` returns `namespace`, `app`, `workerPool`, `clientInstanceId`, and the sorted processor list.

The startup path intentionally keeps script runtime availability checks and tool auto-installation enabled by default. Override `TIKEO_WORKER_SCRIPT_RUNTIME_CHECK`, `TIKEO_WORKER_SCRIPT_AUTO_INSTALL_TOOLS`, or `TIKEO_WORKER_WASM_AUTO_INSTALL` only when testing those failure modes explicitly.

## Spring Boot starter compatibility matrix

The Java SDK intentionally publishes separate starter artifacts for Spring Boot major versions:

- Spring Boot 4.x: `net.tikeo:tikeo-spring-boot-starter`
- Spring Boot 3.x: `net.tikeo:tikeo-spring-boot3-starter`
- Spring Boot 2.x: `net.tikeo:tikeo-spring-boot2-starter`

This demo is the Spring Boot 3.x example and therefore depends on `tikeo-spring-boot3-starter`.

## API-type task management example

In tikeo, `scheduleType: api` means the job is created as an explicit API/SDK/UI-triggered task. It does **not** mean the worker executes an HTTP API call. The Java SDK management client can create, enable/disable, and manually trigger these jobs.

When the demo has a management API key, enable the optional control-plane endpoints:

```bash
(cd examples/java/spring-boot3-worker-demo &&   TIKEO_MANAGEMENT_ENABLED=true   TIKEO_MANAGEMENT_ENDPOINT=http://127.0.0.1:9999   TIKEO_MANAGEMENT_API_KEY=<tk-api-key>   ./gradlew bootRun)
```

Then call:

- `GET /demo/jobs` — list jobs in the configured namespace/app scope.
- `POST /demo/jobs/echo` — create an `api` schedule job for `demo.echo`, disable/enable it, then trigger it through the SDK.
