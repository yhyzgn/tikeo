---
title: Java SDK and Spring Boot Starter
description: Java SDK artifact、Maven/Gradle 依赖选择、Spring Boot、原生 Java 与非 Boot Spring 集成。
---

# Java SDK and Spring Boot Starter

Java SDK 以 Maven Central artifact 发布，group 为 `net.tikeo`。新 Java 服务默认选择 **Spring Boot 4** 的 `net.tikeo:tikeo-spring-boot-starter`。每个服务只添加 **一个** Tikeo 依赖；不要显式添加该依赖已经传递带入的上游 Tikeo 模块。

## 运行时与版本占位符

- Java runtime：Java 17+。
- 仓库 CI 使用 Temurin 21 验证 SDK。
- 将 `${TIKEO_VERSION}` 替换为 README 顶部对应 artifact/package 徽标显示的版本号。
- Java/Maven 使用不带 `v` 的 `${TIKEO_VERSION}`。

## 只选择一个 Java artifact

| Artifact | 什么时候使用 | 依赖行 |
| --- | --- | --- |
| `net.tikeo:tikeo-spring-boot-starter` | 新 Java 服务默认：Spring Boot 4 / Spring Framework 7 自动配置。 | `implementation("net.tikeo:tikeo-spring-boot-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring-boot3-starter` | Spring Boot 3 / Spring Framework 6 自动配置。 | `implementation("net.tikeo:tikeo-spring-boot3-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring-boot2-starter` | Spring Boot 2 / Spring Framework 5 自动配置。 | `implementation("net.tikeo:tikeo-spring-boot2-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo` | 原生 Java Worker、management client、sandbox tooling 或低层 Worker Tunnel 集成。 | `implementation("net.tikeo:tikeo:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring` | 不使用 Boot auto-configuration，手动接线 Spring Framework 7 adapter。 | `implementation("net.tikeo:tikeo-spring:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring6` | 不使用 Boot auto-configuration，手动接线 Spring Framework 6 adapter。 | `implementation("net.tikeo:tikeo-spring6:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring5` | 不使用 Boot auto-configuration，手动接线 Spring Framework 5 adapter。 | `implementation("net.tikeo:tikeo-spring5:${TIKEO_VERSION}")` |

Spring Boot starter 会传递包含匹配的 Spring adapter 和 core SDK。例如 Boot 3 服务只需要依赖 `tikeo-spring-boot3-starter`；不要再额外声明 `tikeo-spring6` 或 `tikeo`。

## Gradle Kotlin DSL

```kotlin
repositories {
    mavenCentral()
}

dependencies {
    // 新 Java 服务默认：Spring Boot 4。
    implementation("net.tikeo:tikeo-spring-boot-starter:${TIKEO_VERSION}")

    // 运行时需要时，从下面替代项里只选择一个：
    // implementation("net.tikeo:tikeo-spring-boot3-starter:${TIKEO_VERSION}") // Spring Boot 3
    // implementation("net.tikeo:tikeo-spring-boot2-starter:${TIKEO_VERSION}") // Spring Boot 2
    // implementation("net.tikeo:tikeo:${TIKEO_VERSION}")                      // 原生 Java
    // implementation("net.tikeo:tikeo-spring:${TIKEO_VERSION}")               // 手动 Spring Framework 7
    // implementation("net.tikeo:tikeo-spring6:${TIKEO_VERSION}")              // 手动 Spring Framework 6
    // implementation("net.tikeo:tikeo-spring5:${TIKEO_VERSION}")              // 手动 Spring Framework 5
}
```

## Maven POM

只复制一个 dependency block。Maven 会从所选 artifact 解析传递的 Tikeo 模块。

```xml
<dependencies>
  <!-- 新 Java 服务默认：Spring Boot 4 / Spring Framework 7。 -->
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring-boot-starter</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>

  <!-- Spring Boot 3 / Spring Framework 6。 -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring-boot3-starter</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- Spring Boot 2 / Spring Framework 5。 -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring-boot2-starter</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- 原生 Java core SDK。 -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- 非 Boot 手动 Spring Framework 7 adapter。 -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- 非 Boot 手动 Spring Framework 6 adapter。 -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring6</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->

  <!-- 非 Boot 手动 Spring Framework 5 adapter。 -->
  <!--
  <dependency>
    <groupId>net.tikeo</groupId>
    <artifactId>tikeo-spring5</artifactId>
    <version>${TIKEO_VERSION}</version>
  </dependency>
  -->
</dependencies>
```

## Spring Boot starter 集成

Boot starter 使用属性配置。它会创建 processor registry、Worker Tunnel client、生命周期 hook、sandbox runner registry 和可选 management client。

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

## 原生 Java core SDK 集成

原生 Java 不使用 `application.yml`。你需要自己构造 `WorkerRegistration`、提供 `TaskProcessor`，并启动 `GrpcTikeoWorkerClient`。

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

原生 Java 使用 Management API 时，直接创建 `HttpTikeoJobClient(endpoint, apiKey, namespace, app)`，API key 从 Secret store 注入。


## Management API 创建并触发任务

Java management helper 位于 core `net.tikeo:tikeo` artifact 的 `net.tikeo.management.*` 包；Spring Boot starter 只是在 `tikeo.management.enabled=true` 时自动配置同一个 client。`HttpTikeoJobClient` 会从 Secret（例如 `TIKEO_MANAGEMENT_API_KEY`）发送 app 级 `x-tikeo-api-key` header，不能绑定浏览器 session、OIDC cookie 或用户 bearer token。`CreateJobRequest.api(...)` 创建 API 调度的 processor job，`TriggerJobRequest.api()` 发送 `triggerType=api` 与默认 `executionMode=single`。

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

广播被建模为另一个显式 helper。`TriggerJobRequest.broadcastApi(...)` 会序列化 `executionMode=broadcast` 和 `broadcastSelector`；只有被选中 Worker 集合都应执行本次 API 触发时才使用。

```java
var selector = new BroadcastSelectorRequest(
    List.of("manual-demo"),
    "us-east-1",
    null,
    Map.of("worker_pool", "java-blue")
);
client.triggerJob(created.id(), TriggerJobRequest.broadcastApi(selector));
```


## Source-backed 参考链接

SDK helper 文档必须锚定到从源码整理出的 API 与协议参考：

- 创建 helper 端点：[`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- 触发 helper 端点：[`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- 实例轮询端点：[`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- 实例日志端点：[`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker 派发消息：[`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## 非 Boot Spring Framework 集成

已有 Spring Framework 应用但不使用 Boot auto-configuration 时，选择 `tikeo-spring`、`tikeo-spring6` 或 `tikeo-spring5`。你需要自己定义 registry 和 Worker client bean。

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

## Spring Boot 属性默认值

全局 Worker 字段见 [配置参考](../reference/configuration#sdk-与-worker-配置)。Spring Boot 额外提供 auto-configuration 开关和 Java sandbox-tool 属性：

| 配置项 | 默认值 | 说明 |
| --- | --- | --- |
| `tikeo.worker.enabled` | `true` | 启用 worker auto-configuration。 |
| `tikeo.worker.auto-startup` | `true` | 随 Spring 应用生命周期启动/停止 worker。 |
| `tikeo.worker.endpoint` | `http://0.0.0.0:9998` | 部署时显式设置为可访问的 Worker Tunnel endpoint。 |
| `tikeo.worker.dry-run` | `false` | 使用 `NoopTikeoWorkerClient`，不打开真实 tunnel。 |
| `tikeo.worker.client-instance-id` | 空 | 可选；为空时 Boot 按 scope/runtime identity 生成并持久化。 |
| `tikeo.worker.state-dir` | 空 → `~/.tikeo/workers` | 生成的 worker instance identity 状态目录。 |
| `tikeo.worker.wasm.auto-install` | `true` | 缺少 Wasmtime 时自动安装。 |
| `tikeo.worker.wasm.install-version` | `latest` | Wasmtime installer 版本。 |
| `tikeo.worker.wasm.install-dir` | 空 → `~/.tikeo/sandbox-tools/wasmtime` | 持久化/缓存可避免重复下载。 |
| `tikeo.worker.scripts.auto-install-tools` | `true` | 缺少脚本工具时自动安装；生产锁定镜像中建议关闭。 |
| `tikeo.worker.scripts.power-shell-install-version` | `7.5.4` | 自动安装 PowerShell Core 的版本。 |
| `tikeo.worker.scripts.power-shell-install-dir` | 空 → `~/.tikeo/sandbox-tools/pwsh` | 持久化/缓存可避免重复 archive 下载。 |
| `tikeo.management.enabled` | `false` | 启用 `TikeoJobClient` auto-configuration。 |
| `tikeo.management.endpoint` | `http://127.0.0.1:9999` | 显式设置；Compose 示例通常将 server HTTP 暴露在 `9090`。 |
| `tikeo.management.api-key` | 空 | App-scoped API key；从 Secret store 注入。 |

Java SDK 低层 PowerShell installer 覆盖项：`TIKEO_POWERSHELL_VERSION`、`TIKEO_POWERSHELL_DOWNLOAD_URL`、`TIKEO_POWERSHELL_INSTALL_TIMEOUT_MILLIS`。

## 部署清单

1. 添加一个且仅一个与运行时匹配的 Java 依赖；新 Boot 4 服务默认使用 `tikeo-spring-boot-starter`。
2. 将 Worker Tunnel endpoint 设置为 Worker 进程可访问的地址。
3. 按路由模型一致设置 namespace、app、cluster、region、labels 和 structured capabilities。
4. Boot 场景中，需要稳定生成 instance identity 时持久化 `state-dir`。
5. 不可变生产镜像中，预安装/缓存 sandbox tools，并按需关闭 auto-install。
6. 如果启用 management client，显式设置 HTTP endpoint，并从 Secret store 注入 API key。
7. 确认 Web 控制台能看到 worker，然后触发路由到 Java capability 的任务并检查日志/结果。

## 本地验证命令

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

## 兼容性规则

Java 模块必须保留明确的 source/resource/test 边界。不要用空模块或 source-set indirection 取代兼容模块。Boot 2、Boot 3、Boot 4 starter 分开存在，是为了让每条兼容线都有真实源码、资源、测试和依赖边界。
