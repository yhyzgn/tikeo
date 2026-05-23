# 101 — Java Spring worker demo runtime fix

## Context
The user reported the Java demo starts and exits immediately, preventing it from appearing in the Worker cluster page.

## Root cause
`SpringWorkerDemoApplication.DemoRunner` called `client.close()` immediately after `client.start()`, so dry-run mode exited and live Worker Tunnel mode would tear down the connection after registration. The documented command also invoked the SDK wrapper without selecting the demo project.

## Objectives
1. Keep the Java Spring worker demo process alive until interrupted.
2. Close the worker client during Spring shutdown rather than immediately after startup.
3. Make the live local endpoint use `127.0.0.1:9998` and document the command that registers with a local tikee server.
4. Verify the demo appears in `/api/v1/workers` with Java/Spring capabilities.

## Expected verification
- `./gradlew test --no-daemon` from `examples/java/spring-worker-demo`
- dry-run `bootRun` remains alive until `timeout` terminates it
- `./gradlew test --no-daemon` from `sdks/java`
- local tikee server + live Java demo show one online worker via `/api/v1/workers`

## Completion notes
- Commit with Lore trailers and push.
