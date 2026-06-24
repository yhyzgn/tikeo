# Tikeo Java SDKs ☕

[🇨🇳 中文 SDK 文档](../../README.zh-CN.md#行为一致的-sdk) · [Full docs site page](../../website/docs/sdks/java-spring-boot.md) · [Shared configuration reference](../../website/docs/reference/configuration.md#sdk-and-worker-configuration)

Tikeo Java SDK artifacts are published to Maven Central under group `net.tikeo`. The default choice for new Java services is **Spring Boot 4** with `net.tikeo:tikeo-spring-boot-starter`, the primary Spring Boot 4.x starter.

Add **exactly one** Tikeo dependency to each service. Starters and adapters already declare their upstream Tikeo modules transitively, so do not also add `tikeo`, `tikeo-spring*`, or another starter unless you are intentionally replacing the selected artifact.

Use `tikeo-spring-boot3-starter` for Spring Boot 3.x projects and `tikeo-spring-boot2-starter` for Spring Boot 2.x projects; these compatibility lines have separate source, resource, dependency, and test boundaries.

## Runtime and version placeholder

- Java runtime baseline: Java 17+.
- Repository CI validates the SDK and demos on Temurin 21.
- Replace `${TIKEO_VERSION}` with the version shown by the matching Maven Central badge at the top of the root [`README.md`](../../README.md).
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

Use `tikeo-spring-boot-starter`, `tikeo-spring-boot3-starter`, or `tikeo-spring-boot2-starter` when your application runs on Spring Boot. The starter creates the processor registry, Worker Tunnel client, lifecycle hook, sandbox runner registries, and optional management client.

### `application.yml`

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
    endpoint: ${TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9090}
    api-key: ${TIKEO_MANAGEMENT_API_KEY:}
    namespace: ${TIKEO_MANAGEMENT_NAMESPACE:default}
    app: ${TIKEO_MANAGEMENT_APP:default}
```

### Annotated processor

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

Use `net.tikeo:tikeo` when you are not using Spring. Plain Java does **not** read `application.yml`; load environment variables, system properties, or your own config file, then build `WorkerRegistration`, provide a `TaskProcessor`, and start `GrpcTikeoWorkerClient` yourself.

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

## Configuration reference

The Worker runtime fields are shared across all SDKs and belong in the global configuration reference, not in a Java-only section. Start with these deployment decisions for every language:

| Field | Default in SDK helpers | Meaning |
| --- | --- | --- |
| `endpoint` | usually `http://127.0.0.1:9998` in demos | Worker Tunnel endpoint reachable from the worker process. Use a Service/LB/DNS name in deployments. |
| `clientInstanceId` / `client_instance_id` | required for core SDK helpers; Java Boot can generate/persist it | Stable client-side hint. The server still assigns the authoritative `worker_id`. |
| `namespace` | `default` | Tenant/environment namespace used for dispatch and management scoping. |
| `app` | `default` | Application scope used for routing and management operations. |
| `cluster` | `local` in core helpers; Java Boot default is `default` | Worker cluster or environment shard. |
| `region` | `local` in core helpers; Java Boot default is `default` | Worker region/zone. |
| `capabilities` | `[]` | Legacy/operator metadata. Prefer structured capabilities for dispatch routing when available. |
| `structuredCapabilities` | empty | SDK processors, script runners, plugin processors, and structured tags used for routing. |
| `labels` | `{}` | Free-form operational metadata such as `worker_pool`, `runtime`, `team`, or `tier`. |
| `election.enabled` | `true` | Worker-cluster master election flag in registration. |
| `election.domain` | blank | Blank means `namespace/app/cluster/region`. |
| `election.priority` | `100` | Deterministic election priority; lower values win. |

See the docs-site [SDK and worker configuration](../../website/docs/reference/configuration.md#sdk-and-worker-configuration) section for the full cross-SDK table and deployment checklist.

### Spring Boot property defaults

| Property | Default | Notes |
| --- | --- | --- |
| `tikeo.worker.enabled` | `true` | Enables worker auto-configuration. |
| `tikeo.worker.auto-startup` | `true` | Starts/stops the worker with the Spring application lifecycle. |
| `tikeo.worker.endpoint` | `http://127.0.0.1:9998` | Local Worker Tunnel endpoint; override to the reachable Service/LB/Gateway URL in deployments. |
| `tikeo.worker.dry-run` | `false` | Uses `NoopTikeoWorkerClient` instead of opening a live tunnel. |
| `tikeo.worker.heartbeat-interval-millis` | `10000` | Worker lease renewal cadence. |
| `tikeo.worker.client-instance-id` | blank | Optional; blank lets Boot generate and persist one per scope/runtime identity. |
| `tikeo.worker.state-dir` | blank → `~/.tikeo/workers` | Directory for generated worker instance identity state. |
| `tikeo.worker.wasm.auto-install` | `true` | Background-prewarms Wasmtime when missing; startup never waits for installer completion. |
| `tikeo.worker.wasm.install-version` | `latest` | Wasmtime installer version. |
| `tikeo.worker.wasm.install-dir` | blank → `~/.tikeo/sandbox-tools/wasmtime` | Persist/cache to avoid repeated downloads. |
| `tikeo.worker.scripts.enabled` | `true` | Enables dynamic script execution through the configured sandbox paths. |
| `tikeo.worker.scripts.container-enabled` | `false` | Enables optional container-backed shell/python/node/powershell runners. |
| `tikeo.worker.scripts.availability-check` | `true` | Probes runtime availability before advertising non-WASM script capabilities. |
| `tikeo.worker.scripts.auto-install-tools` | `true` | Background-prewarms script tooling when absent; disable in locked-down production images. |
| `tikeo.worker.scripts.strict-sandbox-isolation` | `false` | Strict sandbox isolation switch: ignore host PATH tools/interpreters and use only sandbox-tools cache binaries. Env: `TIKEO_WORKER_SCRIPTS_STRICT_SANDBOX_ISOLATION`. |
| `tikeo.worker.scripts.power-shell-install-version` | `7.5.4` | PowerShell Core version for auto-install. |
| `tikeo.worker.scripts.power-shell-install-dir` | blank → `~/.tikeo/sandbox-tools/pwsh` | Persist/cache to avoid repeated archive downloads. |
| `tikeo.worker.scripts.tool-install-timeout-millis` | `120000` | Background script tool installer timeout; failure is logged and never fails Spring startup. |
| `tikeo.management.enabled` | `false` | Enables `TikeoJobClient` auto-configuration. |
| `tikeo.management.endpoint` | `http://127.0.0.1:9090` | HTTP Management endpoint; override to your Server API URL. |
| `tikeo.management.api-key` | blank | App-scoped API key; inject from a Secret store. |
| `tikeo.management.namespace` | `default` | Management namespace scope. |
| `tikeo.management.app` | `default` | Management app scope. |

Low-level PowerShell installer overrides recognized by the Java SDK: `TIKEO_POWERSHELL_VERSION`, `TIKEO_POWERSHELL_DOWNLOAD_URL`, and `TIKEO_POWERSHELL_INSTALL_TIMEOUT_MILLIS`.

## Logging

The Java SDK uses SLF4J for SDK diagnostics and `TaskContext` for task instance logs. Configure the host application logging backend for console plus file output, for example with Logback:

```xml
<appender name="FILE" class="ch.qos.logback.core.FileAppender">
  <file>logs/tikeo-sdk.log</file>
  <encoder><pattern>%d %-5level %logger - %msg%n</pattern></encoder>
</appender>
<logger name="net.tikeo" level="INFO" />
```

Operational cautions:

- Keep SDK diagnostics at INFO in production.
- Do not capture global stdout/stderr for task logs.
- API keys are bearer secrets; never log them.
- Leave sandbox runtime checks enabled so script-capable workers fail closed.

## Deployment checklist

1. Add exactly one Java dependency that matches your runtime; default to `tikeo-spring-boot-starter` for new Boot 4 services.
2. Set the Worker Tunnel endpoint to the address reachable from the worker process.
3. Set namespace, app, cluster, region, labels, and structured capabilities consistently with your routing model.
4. Persist `tikeo.worker.state-dir` when stable generated instance identity matters.
5. Persist sandbox tool cache directories such as `~/.tikeo/sandbox-tools/pwsh` and `~/.tikeo/sandbox-tools/wasmtime` when offline startup or avoiding repeated downloads matters.
6. In immutable production images, preinstall/cache sandbox tools and disable auto-install where required.
7. If management clients are enabled, set the HTTP endpoint explicitly and inject API keys from a Secret store.
8. Verify that the worker appears in the Web console, then trigger a task routed to a Java capability and confirm logs/results.

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
