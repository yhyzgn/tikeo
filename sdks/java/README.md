# tikee Java SDKs

Java SDK packages live under `sdks/java/<sdk-name>/`. This language directory is a Gradle multi-project aggregator; each included SDK module can also be built/tested independently by Gradle task path.

Current packages:

- `tikee/` — native Java integration: Worker Tunnel gRPC client, protocol bindings, task contracts.
- `tikee-spring/` — Spring Framework integration: `@TikeeProcessor` registry and method adapter.
- `tikee-spring-boot-starter/` — Spring Boot integration: auto-configuration, properties, starter-style dependency.

Java SDK uses Gradle and requires JDK 21+. Maven `pom.xml` is intentionally not used. SDK/demo code may use Lombok to reduce boilerplate; Spring beans should prefer constructor injection.

Registration model: Java workers treat tikee-assigned `worker_id` as authoritative. Starter configuration exposes `tikee.worker.client-instance-id` only as an optional stable hint; `GrpcTikeeWorkerClient` reads `WorkerRegistered.worker_id` and uses it for heartbeat/log/result calls.

Validation from repository root:

```bash
(cd sdks/java && ./gradlew test)
(cd sdks/java && ./gradlew :tikee:test)
(cd sdks/java && ./gradlew :tikee-spring:test)
(cd sdks/java && ./gradlew :tikee-spring-boot-starter:test)
```

Spring Boot starter properties:

```yaml
tikee:
  worker:
    dry-run: false # true for local demo without a live tikee
    endpoint: http://0.0.0.0:9998
    client-instance-id: spring-worker
    namespace: default
    app: default
    cluster: local
    region: local
```
