# tikeo Java SDKs

Java SDK packages live under `sdks/java/<sdk-name>/`. This language directory is a Gradle multi-project aggregator: the root `build.gradle.kts` only owns aggregation and shared group/version, while every SDK module owns its own `build.gradle.kts` with module-specific plugins, dependencies, tests, and `maven-publish` publication. This keeps Boot 2/3/4 and Spring 5/6/7 constraints independent and makes each artifact publishable without hidden root-project dependency wiring.

Current packages:

- `tikeo/` — native Java integration: Worker Tunnel gRPC client, protocol bindings, task contracts.
- `tikeo-spring/` — Spring Framework 7 integration for the primary Spring Boot 4 starter.
- `tikeo-spring5/` — Spring Framework 5.3 compatibility adapter for Spring Boot 2.x applications.
- `tikeo-spring6/` — Spring Framework 6.2 compatibility adapter for Spring Boot 3.x applications.
- `tikeo-spring-boot-starter/` — primary Spring Boot 4.x starter.
- `tikeo-spring-boot2-starter/` — compatibility starter for Spring Boot 2.x projects, publishing Boot 2 `spring.factories` metadata.
- `tikeo-spring-boot3-starter/` — compatibility starter for Spring Boot 3.x projects, publishing Boot 3 `AutoConfiguration.imports` metadata.

Java SDK uses Gradle and targets Java 17 bytecode (`--release 17`), so consumers can run it on Java 17+. Maven `pom.xml` is intentionally not used. SDK/demo code may use Lombok to reduce boilerplate; Spring beans should prefer constructor injection. Use the primary starter for Spring Boot 4.x; use the Boot2/Boot3 compatibility starter artifacts when integrating into existing Spring Boot 2.x or 3.x projects.

Registration model: Java workers treat tikeo-assigned `worker_id` as authoritative. The SDK auto-generates and persists a stable `client_instance_id` per namespace/app/cluster/region so reconnects correlate to the same worker identity. `tikeo.worker.client-instance-id` remains an advanced optional override only; normal applications and demos should not set it. `GrpcTikeoWorkerClient` reads `WorkerRegistered.worker_id` and uses it for heartbeat/log/result calls.

Validation from repository root:

```bash
(cd sdks/java && ./gradlew clean test publishToMavenLocal)
(cd sdks/java && ./gradlew :tikeo:test :tikeo:publishToMavenLocal)
(cd sdks/java && ./gradlew :tikeo-spring:test :tikeo-spring:publishToMavenLocal)
(cd sdks/java && ./gradlew :tikeo-spring-boot2-starter:test :tikeo-spring-boot2-starter:publishToMavenLocal)
(cd sdks/java && ./gradlew :tikeo-spring-boot3-starter:test :tikeo-spring-boot3-starter:publishToMavenLocal)
(cd sdks/java && ./gradlew :tikeo-spring-boot-starter:test :tikeo-spring-boot-starter:publishToMavenLocal)
```

Spring Boot starter properties:

```yaml
tikeo:
  worker:
    enabled: true
    auto-startup: true # SmartLifecycle starts/stops the outbound worker client
    dry-run: false # true for local demo without a live tikeo
    endpoint: http://0.0.0.0:9998
    # client-instance-id: optional advanced override; leave blank to let the SDK persist one
    # state-dir: ~/.tikeo/workers
    namespace: default
    app: default
    cluster: local
    region: local
```

The Spring Boot starter creates a `TikeoWorkerLifecycle` bean so the worker client follows the application lifecycle. Set `tikeo.worker.auto-startup=false` when an application wants to start the client manually, or `tikeo.worker.enabled=false` to disable Worker Tunnel beans while keeping processor scanning available.


Job management client:

- `TikeoJobClient` manages jobs in a configured namespace/app scope.
- `CreateJobRequest.api(...)` creates an `api` schedule job, meaning explicit API/SDK/UI-triggered. It does **not** mean the task performs an HTTP API call.
- `enableJob`, `disableJob`, `updateJob`, `deleteJob`, and `triggerJob(..., TriggerJobRequest.api())` cover the common control-plane lifecycle.
