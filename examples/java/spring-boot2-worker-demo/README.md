# Java Spring Boot 2.x Worker Demo

Standard Spring Boot Web demo for `sdks/java/tikee-spring-boot2-starter`, validating the compatibility starter in a Spring Boot 2.x application.

Build, test, and run independently from the repository root. Use the committed Gradle Wrapper; the demo targets Java 17 bytecode and runs on Spring Boot 2.x to validate the matching tikee starter artifact.

```bash
(cd examples/java/spring-boot2-worker-demo && ./gradlew test)
(cd examples/java/spring-boot2-worker-demo && ./gradlew run)
```

The demo test suite covers:

- `EchoProcessorTest` — plain unit test for the example processor behavior.
- `SpringWorkerDemoApplicationTest` — Spring Boot context test for dry-run worker identity, lifecycle startup, capabilities/labels, and `@TikeeProcessor` registry invocation.
- `SpringWorkerDemoDisabledTest` — verifies `tikee.worker.enabled=false` keeps processor discovery but does not create a Worker client.
- `SpringBootStarterCompatibilityMatrixTest` — verifies this demo uses `tikee-spring-boot2-starter` and that the Java SDK exposes real Boot 2, Boot 3, and Boot 4 starter modules with concrete `src` trees.

The demo is a normal embedded-web Spring Boot application. `run` stays online through the web server, not a custom blocking runner. It exposes `GET /demo/health` and `GET /demo/processors` on `TIKEE_DEMO_SERVER_PORT` (default `18082`).

The demo configures `client-instance-id` to `spring-boot2-worker-demo` by default so multiple Java demos in the same namespace/app still register as separate workers. Override `TIKEE_WORKER_CLIENT_INSTANCE_ID` only when intentionally testing worker identity reuse. The demo defaults to `tikee.worker.dry-run=false`, so `run` connects to the live Worker Tunnel at `TIKEE_WORKER_ENDPOINT` and should appear in the Worker cluster page after registration. Start tikee with `config/dev.toml`, then run:

```bash
(cd examples/java/spring-boot2-worker-demo && TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 TIKEE_DEMO_SERVER_PORT=18082 ./gradlew run)
```


For the manual acceptance defaults, prefer the wrapper script because it sets the demo scope and management scope consistently:

```bash
(cd examples/java/spring-boot2-worker-demo && ./scripts/run-demo-worker.sh)
```

For local UI-only startup without a tikee server, explicitly set `TIKEE_WORKER_DRY_RUN=true`; dry-run workers do not register with the server and will not appear in the Worker cluster page.


## Manual integration acceptance scope

By default `scripts/run-demo-worker.sh` starts this demo in the same scope used by `scripts/dev-integration-seed.sh` and `scripts/start-java-demo-workers.sh`:

- scope: `dev-alpha/orders/boot2-blue`
- advertised processors: `demo.echo, demo.context, demo.bytes, demo.heartbeat, demo.report, demo.workflow.step, demo.fail, sql:billing.sql-sync`
- health check: `GET /demo/health` returns `namespace`, `app`, `workerPool`, `clientInstanceId`, and the sorted processor list.

The startup path intentionally keeps script runtime availability checks and tool auto-installation enabled by default. Override `TIKEE_WORKER_SCRIPT_RUNTIME_CHECK`, `TIKEE_WORKER_SCRIPT_AUTO_INSTALL_TOOLS`, or `TIKEE_WORKER_WASM_AUTO_INSTALL` only when testing those failure modes explicitly.

## Spring Boot starter compatibility matrix

The Java SDK intentionally publishes separate starter artifacts for Spring Boot major versions:

- Spring Boot 4.x: `com.yhyzgn.tikee:tikee-spring-boot-starter`
- Spring Boot 3.x: `com.yhyzgn.tikee:tikee-spring-boot3-starter`
- Spring Boot 2.x: `com.yhyzgn.tikee:tikee-spring-boot2-starter`

This demo is the Spring Boot 2.x example and therefore depends on `tikee-spring-boot2-starter`.

## API-type task management example

In tikee, `scheduleType: api` means the job is created as an explicit API/SDK/UI-triggered task. It does **not** mean the worker executes an HTTP API call. The Java SDK management client can create, enable/disable, and manually trigger these jobs.

When the demo has a management API key, enable the optional control-plane endpoints:

```bash
(cd examples/java/spring-boot2-worker-demo &&   TIKEE_MANAGEMENT_ENABLED=true   TIKEE_MANAGEMENT_ENDPOINT=http://127.0.0.1:9999   TIKEE_MANAGEMENT_API_KEY=<tk-api-key>   ./gradlew run)
```

Then call:

- `GET /demo/jobs` — list jobs in the configured namespace/app scope.
- `POST /demo/jobs/echo` — create an `api` schedule job for `demo.echo`, disable/enable it, then trigger it through the SDK.
