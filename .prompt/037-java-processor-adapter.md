# 037 — Java TikeoProcessor method adapter

## Context
Java Core SDK now has a real `GrpcTikeoWorkerClient` and generated Worker Tunnel protobuf bindings. Spring Boot auto-configuration creates the real client by default, with `tikeo.worker.dry-run=true` available for demos/tests.

## Current capabilities
- Java SDK registration sends only `client_instance_id`; `WorkerRegistered.worker_id` is the authoritative id.
- Java client sends heartbeats, emits task logs, accepts dispatches, and reports task results.
- Spring demo defaults to dry-run and can be switched to live Worker Tunnel through configuration.

## Next goal
Make Spring Boot `@TikeoProcessor` annotations execute real dispatched tasks instead of the current default success processor.

## Required work
1. Extend `TikeoProcessorRegistry` to store invocable method metadata, not only bean references.
2. Add a processor adapter that maps `DispatchTask` to the right annotated method by processor/job metadata once protocol payload convention is defined.
3. Support method signatures such as `TaskOutcome process(TaskContext)`, `String process(String payload)`, and `void process(TaskContext)` with safe exception-to-failure mapping.
4. Add tests for annotation discovery, duplicate processor names, invocation success/failure, and Spring autoconfig wiring.
5. Update Java demo to include at least one processor method that is exercised in dry-run/unit tests.

## Validation
- `./sdks/java/gradlew -p sdks/java test` or cached Gradle equivalent if wrapper download fails.
- `./sdks/java/gradlew -p examples/java/spring-worker-demo test bootRun --args='--spring.main.web-application-type=none'`.
- Run Rust/backend checks if shared proto or server behavior changes.
