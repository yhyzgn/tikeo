---
title: Java SDK and Spring Boot Starter
description: Java SDK artifacts, Maven/Gradle dependency selection, Spring Boot, plain Java, and non-Boot Spring integration.
---

# Java SDK and Spring Boot Starter

The Java SDK is published as Maven Central artifacts under group `net.tikeo`. The default choice for new Java services is **Spring Boot 4** with `net.tikeo:tikeo-spring-boot-starter`. A service should add **one** Tikeo dependency only; do not explicitly add transitive upstream Tikeo modules.

## Runtime and version placeholder

- Java runtime: Java 17+.
- Repository CI validates the SDK on Temurin 21.
- Replace `${TIKEO_VERSION}` with the version shown by the README package badge for the artifact you are installing.
- Java/Maven uses `${TIKEO_VERSION}` without a leading `v`.

## Pick exactly one Java artifact

| Artifact | Use it when... | Dependency line |
| --- | --- | --- |
| `net.tikeo:tikeo-spring-boot-starter` | Default for new Java services: Spring Boot 4 / Spring Framework 7 auto-configuration. | `implementation("net.tikeo:tikeo-spring-boot-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring-boot3-starter` | Spring Boot 3 / Spring Framework 6 auto-configuration. | `implementation("net.tikeo:tikeo-spring-boot3-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring-boot2-starter` | Spring Boot 2 / Spring Framework 5 auto-configuration. | `implementation("net.tikeo:tikeo-spring-boot2-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo` | Plain Java worker, management client, sandbox tooling, or low-level Worker Tunnel integration. | `implementation("net.tikeo:tikeo:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring` | Manual Spring Framework 7 adapter without Boot auto-configuration. | `implementation("net.tikeo:tikeo-spring:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring6` | Manual Spring Framework 6 adapter without Boot auto-configuration. | `implementation("net.tikeo:tikeo-spring6:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring5` | Manual Spring Framework 5 adapter without Boot auto-configuration. | `implementation("net.tikeo:tikeo-spring5:${TIKEO_VERSION}")` |

Spring Boot starters transitively include the matching Spring adapter and core SDK. For example, a Boot 3 service should depend on `tikeo-spring-boot3-starter` only; it should not also declare `tikeo-spring6` or `tikeo`.

## Gradle Kotlin DSL

```kotlin
repositories {
    mavenCentral()
}

dependencies {
    // Default for new Java services: Spring Boot 4.
    implementation("net.tikeo:tikeo-spring-boot-starter:${TIKEO_VERSION}")

    // Pick exactly one of these alternatives instead when your runtime requires it:
    // implementation("net.tikeo:tikeo-spring-boot3-starter:${TIKEO_VERSION}") // Spring Boot 3
    // implementation("net.tikeo:tikeo-spring-boot2-starter:${TIKEO_VERSION}") // Spring Boot 2
    // implementation("net.tikeo:tikeo:${TIKEO_VERSION}")                      // plain Java
    // implementation("net.tikeo:tikeo-spring:${TIKEO_VERSION}")               // manual Spring Framework 7
    // implementation("net.tikeo:tikeo-spring6:${TIKEO_VERSION}")              // manual Spring Framework 6
    // implementation("net.tikeo:tikeo-spring5:${TIKEO_VERSION}")              // manual Spring Framework 5
}
```

## Maven POM

Copy exactly one dependency block. Maven resolves transitive Tikeo modules from the selected artifact.

```xml
<dependencies>
  <!-- Default for new Java services: Spring Boot 4 / Spring Framework 7. -->
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring-boot-starter</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>

  <!-- Spring Boot 3 / Spring Framework 6. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring-boot3-starter</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- Spring Boot 2 / Spring Framework 5. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring-boot2-starter</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- Plain Java core SDK. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- Manual Spring Framework 7 adapter without Boot auto-configuration. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- Manual Spring Framework 6 adapter without Boot auto-configuration. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring6</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- Manual Spring Framework 5 adapter without Boot auto-configuration. -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring5</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->
</dependencies>
```

## Spring Boot starter integration

Boot starters are property-driven. They create the processor registry, Worker Tunnel client, lifecycle hook, sandbox runner registries, and optional management client.

```yaml
tikeo:
  worker:
    enabled: true
    auto-startup: true
    dry-run: ${TIKEO_WORKER_DRY_RUN:false}
    endpoint: ${TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}
    client-instance-id: ${TIKEO_WORKER_CLIENT_INSTANCE_ID:}
    state-dir: ${TIKEO_WORKER_STATE_DIR:}
    namespace: ${TIKEO_WORKER_NAMESPACE:default}
    app: ${TIKEO_WORKER_APP:default}
    cluster: ${TIKEO_WORKER_CLUSTER:default}
    region: ${TIKEO_WORKER_REGION:default}
    capabilities: [java, spring-boot]
    labels:
      worker_pool: ${TIKEO_WORKER_POOL:java-blue}
      runtime: java

  management:
    enabled: ${TIKEO_MANAGEMENT_ENABLED:false}
    endpoint: ${TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9999}
    api-key: ${TIKEO_MANAGEMENT_API_KEY:}
    namespace: ${TIKEO_MANAGEMENT_NAMESPACE:default}
    app: ${TIKEO_MANAGEMENT_APP:default}
```

```java
import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TikeoProcessor;
import org.springframework.stereotype.Component;

@Component
public final class BillingProcessors {
    @TikeoProcessor("billing.reconcile")
    public TaskOutcome reconcile(TaskContext context, String payload) {
        context.logInfo("billing reconcile started");
        return new TaskOutcome(true, "processed:" + payload);
    }
}
```

## Plain Java core SDK integration

Plain Java does not use `application.yml`. Build `WorkerRegistration`, provide a `TaskProcessor`, then start `GrpcTikeoWorkerClient` yourself.

```java
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TaskProcessor;
import net.tikeo.worker.WorkerCapabilitySet;
import net.tikeo.worker.WorkerClusterElection;
import net.tikeo.worker.WorkerRegistration;
import net.tikeo.worker.client.GrpcTikeoWorkerClient;
import java.time.Duration;
import java.util.List;
import java.util.Map;

public final class TikeoPlainJavaWorker {
    public static void main(String[] args) {
        var registration = new WorkerRegistration(
            "orders-java-1",
            "default",
            "orders",
            "local",
            "local",
            List.of("java"),
            new WorkerCapabilitySet(
                List.of("java"),
                List.of("billing.reconcile"),
                List.of(),
                List.of()
            ),
            WorkerClusterElection.enabledByDefault(),
            Map.of("worker_pool", "java-core")
        );

        TaskProcessor processor = context -> {
            context.logInfo("plain Java task started");
            return new TaskOutcome(true, "ok:" + context.processorName());
        };

        var client = new GrpcTikeoWorkerClient(
            System.getenv().getOrDefault("TIKEO_WORKER_ENDPOINT", "http://127.0.0.1:9998"),
            registration,
            processor,
            Duration.ofSeconds(10)
        );
        Runtime.getRuntime().addShutdownHook(new Thread(client::close));
        client.start();
    }
}
```

For management API access from plain Java, create `HttpTikeoJobClient(endpoint, apiKey, namespace, app)` directly and inject the API key from your Secret store.


## Management API create + trigger

Java management helpers live in the core `net.tikeo:tikeo` artifact under `net.tikeo.management.*`; Spring Boot starters only auto-configure the same client when `tikeo.management.enabled=true`. `HttpTikeoJobClient` sends the app-scoped `x-tikeo-api-key` header from a Secret such as `TIKEO_MANAGEMENT_API_KEY`. It must not be wired to a browser session, OIDC cookie, or user-scoped bearer token. `CreateJobRequest.api(...)` creates an API-scheduled processor job, and `TriggerJobRequest.api()` sends `triggerType=api` with the default `executionMode=single`.

```java
import net.tikeo.management.client.HttpTikeoJobClient;
import net.tikeo.management.client.TikeoJobClient;
import net.tikeo.management.model.BroadcastSelectorRequest;
import net.tikeo.management.model.CreateJobRequest;
import net.tikeo.management.model.TriggerJobRequest;
import java.util.List;
import java.util.Map;

String endpoint = System.getenv().getOrDefault(
    "TIKEO_MANAGEMENT_ENDPOINT",
    "http://127.0.0.1:9090"
);
String apiKey = System.getenv("TIKEO_MANAGEMENT_API_KEY");
TikeoJobClient client = new HttpTikeoJobClient(endpoint, apiKey, "dev-alpha", "orders");

var created = client.createJob(CreateJobRequest.api("java-echo-api", "demo.echo"));
var instance = client.triggerJob(created.id(), TriggerJobRequest.api());

if (!"api".equals(instance.triggerType()) || !"single".equals(instance.executionMode())) {
    throw new IllegalStateException("unexpected trigger response");
}
```

Broadcast is intentionally modeled as a different helper. `TriggerJobRequest.broadcastApi(...)` serializes `executionMode=broadcast` and a `broadcastSelector`; use it only when the selected worker set should all receive the API-triggered execution.

```java
var selector = new BroadcastSelectorRequest(
    List.of("manual-demo"),
    "us-east-1",
    null,
    Map.of("worker_pool", "java-blue")
);
client.triggerJob(created.id(), TriggerJobRequest.broadcastApi(selector));
```


## Source-backed reference links

Keep SDK helper docs anchored to source-derived API and protocol references:

- Create helper endpoint: [`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- Trigger helper endpoint: [`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- Instance polling endpoint: [`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- Instance log endpoint: [`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker dispatch message: [`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## Non-Boot Spring Framework integration

Use `tikeo-spring`, `tikeo-spring6`, or `tikeo-spring5` when you have a Spring Framework application without Boot auto-configuration. You must define the registry and Worker client beans yourself.

```java
import net.tikeo.spring.processor.TikeoProcessorRegistry;
import net.tikeo.spring.worker.SpringTikeoTaskProcessor;
import net.tikeo.worker.WorkerClusterElection;
import net.tikeo.worker.WorkerRegistration;
import net.tikeo.worker.client.GrpcTikeoWorkerClient;
import net.tikeo.worker.client.TikeoWorkerClient;
import java.time.Duration;
import java.util.List;
import java.util.Map;
import org.springframework.context.ApplicationContext;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

@Configuration
class TikeoSpringWorkerConfiguration {
    @Bean
    TikeoProcessorRegistry tikeoProcessorRegistry() {
        return new TikeoProcessorRegistry();
    }

    @Bean(initMethod = "start", destroyMethod = "close")
    TikeoWorkerClient tikeoWorkerClient(
        ApplicationContext applicationContext,
        TikeoProcessorRegistry registry
    ) {
        registry.scanExistingBeans(applicationContext);
        var registration = new WorkerRegistration(
            "orders-spring-1",
            "default",
            "orders",
            "local",
            "local",
            List.of("java", "spring"),
            registry.workerCapabilities(),
            WorkerClusterElection.enabledByDefault(),
            Map.of("worker_pool", "spring-manual")
        );
        return new GrpcTikeoWorkerClient(
            System.getenv().getOrDefault("TIKEO_WORKER_ENDPOINT", "http://127.0.0.1:9998"),
            registration,
            new SpringTikeoTaskProcessor(registry),
            Duration.ofSeconds(10)
        );
    }
}
```

## Spring Boot property defaults

The global Worker fields are documented in [Configuration reference](../reference/configuration#sdk-and-worker-configuration). Spring Boot adds auto-configuration switches and Java sandbox-tool properties:

| Property | Default | Notes |
| --- | --- | --- |
| `tikeo.worker.enabled` | `true` | Enables worker auto-configuration. |
| `tikeo.worker.auto-startup` | `true` | Starts/stops the worker with the Spring application lifecycle. |
| `tikeo.worker.endpoint` | `http://0.0.0.0:9998` | Set explicitly to the reachable Worker Tunnel endpoint in deployments. |
| `tikeo.worker.dry-run` | `false` | Uses `NoopTikeoWorkerClient` instead of opening a live tunnel. |
| `tikeo.worker.client-instance-id` | blank | Optional; blank lets Boot generate and persist one per scope/runtime identity. |
| `tikeo.worker.state-dir` | blank → `~/.tikeo/workers` | Directory for generated worker instance identity state. |
| `tikeo.worker.wasm.auto-install` | `true` | Installs Wasmtime automatically when missing. |
| `tikeo.worker.wasm.install-version` | `latest` | Wasmtime installer version. |
| `tikeo.worker.wasm.install-dir` | blank → `~/.tikeo/sandbox-tools/wasmtime` | Persist/cache to avoid repeated downloads. |
| `tikeo.worker.scripts.auto-install-tools` | `true` | Installs script tooling when absent; disable in locked-down production images. |
| `tikeo.worker.scripts.power-shell-install-version` | `7.5.4` | PowerShell Core version for auto-install. |
| `tikeo.worker.scripts.power-shell-install-dir` | blank → `~/.tikeo/sandbox-tools/pwsh` | Persist/cache to avoid repeated archive downloads. |
| `tikeo.management.enabled` | `false` | Enables `TikeoJobClient` auto-configuration. |
| `tikeo.management.endpoint` | `http://127.0.0.1:9999` | Set explicitly; Compose examples usually expose server HTTP on `9090`. |
| `tikeo.management.api-key` | blank | App-scoped API key; inject from a Secret store. |

Low-level PowerShell installer overrides recognized by the Java SDK: `TIKEO_POWERSHELL_VERSION`, `TIKEO_POWERSHELL_DOWNLOAD_URL`, and `TIKEO_POWERSHELL_INSTALL_TIMEOUT_MILLIS`.

## Deployment checklist

1. Add exactly one Java dependency that matches your runtime; default to `tikeo-spring-boot-starter` for new Boot 4 services.
2. Set the Worker Tunnel endpoint to the address reachable from the worker process.
3. Set namespace, app, cluster, region, labels, and structured capabilities consistently with your routing model.
4. For Boot, persist `state-dir` when stable generated instance identity matters.
5. In immutable production images, preinstall/cache sandbox tools and disable auto-install where required.
6. If management clients are enabled, set the HTTP endpoint explicitly and inject API keys from a Secret store.
7. Verify that the worker appears in the Web console, then trigger a task routed to a Java capability and confirm logs/results.

## Local verification commands

```bash
cd sdks/java
./gradlew test --no-daemon
./gradlew jar sourcesJar --no-daemon
```

```bash
cd examples/java/spring-boot2-worker-demo && ./gradlew test --no-daemon
cd examples/java/spring-boot3-worker-demo && ./gradlew test --no-daemon
cd examples/java/spring-boot4-worker-demo && ./gradlew test --no-daemon
```

## Compatibility rule

Java modules must keep explicit source/resource/test boundaries. Do not replace compatibility modules with empty source-set indirection. The separate Boot 2, Boot 3, and Boot 4 starters exist so each compatibility line has real source, resources, tests, and dependency boundaries.
