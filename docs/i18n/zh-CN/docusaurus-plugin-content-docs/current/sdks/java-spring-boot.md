---
title: Java SDK and Spring Boot Starter
description: Java SDK artifact、Spring Boot Starter、Management helper 与 Worker demo 的 operator-grade 验收入口。
---

# Java SDK and Spring Boot Starter

Java SDK 位于 `sdks/java`，Spring Boot demo 位于 `examples/java/spring-boot{2,3,4}-worker-demo`。本文以 starter properties、auto-configuration、core management client、`@TikeoProcessor` adapter 和 demo 源码为事实来源。Java Worker 同样是 **outbound-only**：应用进程通过 `GrpcTikeoWorkerClient` 主动连接 Worker Tunnel，注册 capabilities，接收 `DispatchTask`，并通过 tunnel 回传 task log/result；不要把业务 Worker 写成 inbound Service。demo 中的 `/demo/*` 端点只是运维演示和 management 示例，不是 Worker 派发入口。

## 依赖坐标

Java group 来自 `sdks/java/build.gradle.kts`：`group = "net.tikeo"`，版本来自 Gradle property `tikeoVersion`。每个服务只选择一个 Tikeo artifact；starter 会传递所需 adapter/core SDK。

| Artifact | 使用场景 | Gradle Kotlin DSL |
| --- | --- | --- |
| `net.tikeo:tikeo-spring-boot-starter` | 新服务默认，Spring Boot 4 / Spring Framework 7 | `implementation("net.tikeo:tikeo-spring-boot-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring-boot3-starter` | Spring Boot 3 / Spring Framework 6 | `implementation("net.tikeo:tikeo-spring-boot3-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring-boot2-starter` | Spring Boot 2 / Spring Framework 5 | `implementation("net.tikeo:tikeo-spring-boot2-starter:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo` | 原生 Java Worker、Management helper、sandbox/wasm 工具 | `implementation("net.tikeo:tikeo:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring` | 非 Boot，手动接线 Spring Framework 7 adapter | `implementation("net.tikeo:tikeo-spring:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring6` | 非 Boot，手动接线 Spring Framework 6 adapter | `implementation("net.tikeo:tikeo-spring6:${TIKEO_VERSION}")` |
| `net.tikeo:tikeo-spring5` | 非 Boot，手动接线 Spring Framework 5 adapter | `implementation("net.tikeo:tikeo-spring5:${TIKEO_VERSION}")` |

Maven 同样只复制一个 dependency block，例如 Boot 3：

```xml
<dependency>
  <groupId>net.tikeo</groupId>
  <artifactId>tikeo-spring-boot3-starter</artifactId>
  <version>${TIKEO_VERSION}</version>
</dependency>
```

## Spring Boot 属性默认值

`TikeoWorkerProperties` 的 prefix 是 `tikeo.worker`。默认值：`enabled=true`，`auto-startup=true`，`endpoint 使用 all-interfaces host 和端口 `9998` 的默认值`，`dry-run=false`，`heartbeat-interval-millis=10000`，`client-instance-id` 为空，`state-dir` 为空时使用 `~/.tikeo/workers` 生成稳定 identity，`namespace="default"`，`app="default"`，`cluster="default"`，`region="default"`，`capabilities=[]`，`labels={}`。election 默认 `enabled=true`、`domain=""`、`priority=100`。WASM 默认 `auto-install=true`、`install-version="latest"`、`install-timeout-millis=120000`。scripts 默认 `enabled=true`、`container-enabled=false`、`availability-check=true`、`runtime-command=""`、`auto-install-tools=true`，SRT/Deno/ripgrep 等 install version 多为 `latest`，PowerShell 默认 `7.5.4`，WasmEdge auto install 默认 false。

`TikeoManagementProperties` 的 prefix 是 `tikeo.management`。默认值：`enabled=false`，`endpoint="http://127.0.0.1:9999"`，`api-key=""`，`namespace="default"`，`app="default"`。当 `tikeo.management.enabled=true` 时，auto-configuration 创建 `TikeoJobClient` bean，具体实现是 `HttpTikeoJobClient(endpoint, apiKey, namespace, app)`。

```yaml
tikeo:
  worker:
    enabled: true
    auto-startup: true
    dry-run: ${TIKEO_WORKER_DRY_RUN:false}
    endpoint: ${TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}
    client-instance-id: ${TIKEO_WORKER_CLIENT_INSTANCE_ID:}
    state-dir: ${TIKEO_WORKER_STATE_DIR:}
    namespace: ${TIKEO_WORKER_NAMESPACE:dev-alpha}
    app: ${TIKEO_WORKER_APP:orders}
    cluster: ${TIKEO_WORKER_CLUSTER:local}
    region: ${TIKEO_WORKER_REGION:local}
    capabilities: [java, spring-boot]
    labels:
      worker_pool: ${TIKEO_WORKER_POOL:boot3-blue}
      runtime: java
  management:
    enabled: ${TIKEO_MANAGEMENT_ENABLED:false}
    endpoint: ${TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9999}
    api-key: ${TIKEO_MANAGEMENT_API_KEY:}
    namespace: ${TIKEO_MANAGEMENT_NAMESPACE:dev-alpha}
    app: ${TIKEO_MANAGEMENT_APP:orders}
```

## @TikeoProcessor 与最小 Spring Worker

Spring adapter 会扫描 Spring beans 中的 `@TikeoProcessor` 方法并注册到 `TikeoProcessorRegistry`。`TikeoWorkerAutoConfiguration` 创建 `WorkerRegistration`，合并 `properties.capabilities`、registry 中的 SDK/plugin processors、script runner registry 和 wasm runner registry，然后创建 `GrpcTikeoWorkerClient`；如果 `tikeo.worker.dry-run=true`，则创建 `NoopTikeoWorkerClient`。最小业务 Worker 只需要一个 Spring Boot 应用和 processor bean：

```java
import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TikeoProcessor;
import org.springframework.stereotype.Component;

@Component
public final class BillingProcessors {
    @TikeoProcessor("demo.echo")
    public TaskOutcome echo(TaskContext context, String payload) {
        context.logInfo("[demo.echo] received payload='" + payload + "'");
        return new TaskOutcome(true, "echo:" + payload);
    }
}
```

plugin processor 使用源码中的属性：

```java
import net.tikeo.processor.TikeoProcessor;
import net.tikeo.processor.TikeoProcessorKind;

@TikeoProcessor(value = "billing.sql-sync", kind = TikeoProcessorKind.PLUGIN, pluginType = "sql")
public String run(TaskContext context, String payload) {
    context.logInfo("[billing.sql-sync] plugin SQL processor received payload='" + payload + "'");
    return "sql-plugin-ok:" + payload;
}
```

demo processor 名称来自 `examples/java/spring-boot3-worker-demo/src/main/java/.../processor`：`demo.echo`、`demo.context`、`demo.bytes`、`demo.heartbeat`、`demo.report`、`demo.workflow.step`、`demo.fail`、`demo.exception` 和 plugin `billing.sql-sync`。这些是 Worker dispatch 能力，不是 HTTP handler。`DemoInfoController` 和 `DemoJobManagementController` 只用于检查注册状态和演示 job management。

## 原生 Java Worker

不使用 Boot 时，直接使用 core SDK：构造 `WorkerRegistration`、`WorkerCapabilitySet`、`TaskProcessor` 和 `GrpcTikeoWorkerClient`。这条路径没有 `application.yml` 属性绑定，也不会自动扫描 `@TikeoProcessor`。

```java
var registration = new WorkerRegistration(
    "orders-java-1",
    "dev-alpha",
    "orders",
    "local",
    "local",
    List.of("java"),
    new WorkerCapabilitySet(List.of("java"), List.of("demo.echo"), List.of(), List.of()),
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
client.start();
```

## Management API 与管理客户端凭证

Java core Management helper 是 `HttpTikeoJobClient`，接口是 `TikeoJobClient`；其他语言页的 `ManagementClient` 在 Java 中对应这个 helper。构造函数 `HttpTikeoJobClient(endpoint, apiKey, namespace, app)` 会 trim endpoint，空 namespace/app 默认 `default`，请求 timeout 30 秒，connect timeout 10 秒，请求头固定包含 `x-tikeo-api-key` 与 `accept: application/json`。凭证应来自 `TIKEO_MANAGEMENT_API_KEY`；不要使用浏览器 session、OIDC cookie、人类 bearer token 或 demo 默认值作为生产凭证。

源码 helper：`CreateJobRequest.api(name, processorName)` 创建 `scheduleType=api` processor job；`CreateJobRequest.apiPlugin(name, processorType, processorName)` 创建 plugin job；`CreateJobRequest.apiScript(name, scriptId)` 创建 script job；`TriggerJobRequest.api()` 发送 `triggerType=api` 与 `executionMode=single`；`TriggerJobRequest.broadcastApi(new BroadcastSelectorRequest(...))` 发送 `executionMode=broadcast` 与 `broadcastSelector`。

```java
import net.tikeo.management.client.HttpTikeoJobClient;
import net.tikeo.management.model.BroadcastSelectorRequest;
import net.tikeo.management.model.CreateJobRequest;
import net.tikeo.management.model.TriggerJobRequest;
import java.util.List;
import java.util.Map;

var client = new HttpTikeoJobClient(
    System.getenv().getOrDefault("TIKEO_MANAGEMENT_ENDPOINT", "http://127.0.0.1:9999"),
    System.getenv("TIKEO_MANAGEMENT_API_KEY"),
    "dev-alpha",
    "orders"
);
var created = client.createJob(CreateJobRequest.api("java-echo-api", "demo.echo"));
var instance = client.triggerJob(created.id(), TriggerJobRequest.api());
if (!"api".equals(instance.triggerType()) || !"single".equals(instance.executionMode())) {
    throw new IllegalStateException("unexpected trigger response");
}
var selector = new BroadcastSelectorRequest(
    List.of("manual-demo"),
    "us-east-1",
    null,
    Map.of("worker_pool", "boot3-blue")
);
client.triggerJob(created.id(), TriggerJobRequest.broadcastApi(selector));
```

参考锚点：[`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)、[`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)、[`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)、[`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)、[`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)。

## Demo 与运维边界

Spring Boot 3 demo 的 `application.yml` 将 `tikeo.worker.endpoint` 默认设为 `http://127.0.0.1:9998`，`client-instance-id` 默认 `spring-boot3-worker-demo-${HOSTNAME:local}`，namespace/app 默认 `dev-alpha`/`orders`，label `worker_pool=boot3-blue`、`runtime=java`、`demo=spring-boot3-worker-demo`。`tikeo.management.enabled` 在 demo 中默认 true，生产服务应按最小权限显式开启，并从 Secret 注入 `TIKEO_MANAGEMENT_API_KEY`。脚本 runner 默认开启 WASM/SRT 工具解析，但 container scripts 默认关闭；只有可用运行时会被注册进 structured capabilities。

## 失败与异常 demo

Spring Boot demo 区分预期业务失败和运行时异常。`demo.fail` 返回 failed `TaskOutcome`；`demo.exception` 抛出 `IllegalStateException`。Spring adapter 会通过 `TaskContext.logError` 记录 Java 堆栈，因此 live 派发时异常栈会出现在实例日志和通知卡片跳转的公开执行控制台中。

## 运维依据与排错边界

核对 Java 集成时，先读对应 starter 的 `TikeoWorkerProperties` 与 `TikeoManagementProperties`，再读 `TikeoWorkerAutoConfiguration` 如何创建 `TikeoProcessorRegistry`、`ScriptRunnerRegistry`、`WasmRunnerRegistry`、`WorkerRegistration` 和 `GrpcTikeoWorkerClient`。`TikeoProcessorRegistry` 只扫描 Spring bean 上的 `@TikeoProcessor`，所以 processor 名称、`TikeoProcessorKind.PLUGIN` 和 `pluginType` 才是调度事实。`examples/java/spring-boot3-worker-demo` 同时包含 processor、dry-run 测试、management controller 和 `/demo/health` 运维检查；这些 HTTP endpoint 证明应用状态，不是 Worker inbound dispatch。排错顺序应是：确认 starter artifact 与 Spring Boot 版本匹配，确认 properties 最终值，确认 registry handlers，确认 Worker registration 的 structured capabilities，最后再检查 Management API 请求与 job instance 日志。

## 生产上线检查

上线前确认 Java 服务选择了唯一 starter artifact，避免同时引入 Boot 2/3/4 starter 或手动 Spring adapter 造成重复 bean。`client-instance-id` 可以显式设置或由 state dir 持久化生成，但 Worker 权威身份仍是 Server 注册 ack 返回的 worker id、generation、lease 和 fencing token。多副本部署时，用 namespace、app、cluster、region、capabilities 和 labels 表达调度边界；不要把 `/demo/*` 运维端点暴露成任务入口。每次新增 `@TikeoProcessor` 方法、plugin type 或脚本能力，都应被视为调度面变更并进入发布评审。Management API key 只放在 Secret 配置中，不能写进 `application.yml` 的生产值、镜像层、Actuator 输出或普通业务日志。

生产观测还应覆盖 Spring lifecycle 启停、registry handlers 数量、tunnel 重连、heartbeat 延迟、任务失败分类和 management 请求错误率。滚动发布时先验证一个实例的 capabilities，再扩容，避免不同 Pod 因属性或工具差异广告出不同能力。

如果使用 Actuator、集中日志或配置中心，还要确认 `tikeo.management.api-key` 被脱敏，`tikeo.worker.labels` 不包含租户秘密，`@TikeoProcessor` 方法不会把原始 payload 全量写入普通日志。灰度阶段保留 dry-run 配置样本，方便在不连接 live tunnel 的情况下复核自动配置和 processor 扫描结果。

对于 Boot 应用，建议把 Worker 专用 profile、Management 专用 profile 和普通 Web profile 分开维护。

这能避免普通 Web 配置把 Worker 连接、processor 扫描或 management 凭证带到不该运行的进程里。

灰度时先只启动一个 Worker 实例。

## 现场验收 runbook

1. SDK/starter 测试：在 `sdks/java` 下运行 `./gradlew test`，确认 core、Spring adapter、Boot 2/3/4 starter 测试都通过。
2. demo dry-run：在对应 demo 目录运行 `TIKEO_WORKER_DRY_RUN=true ./gradlew bootRun`，确认 `NoopTikeoWorkerClient` 启动、`/demo/health` 返回 `connected=true`（dry-run 连接态）、processor 列表包含 `demo.echo` 和 `billing.sql-sync`。
3. live tunnel：启动 Server，设置 `TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998`、namespace/app、cluster/region、`TIKEO_WORKER_POOL=boot3-blue`，关闭 dry-run，确认 Web 控制台出现 outbound Java Worker session。
4. Management 验收：开启 `TIKEO_MANAGEMENT_ENABLED=true`，设置 `TIKEO_MANAGEMENT_ENDPOINT` 与 `TIKEO_MANAGEMENT_API_KEY`，调用 demo `/demo/jobs/echo` 或直接使用 `HttpTikeoJobClient`；确认请求携带 `x-tikeo-api-key`，响应 `triggerType=api` 与 `executionMode=single`。
5. 广播验收：只在需要扇出时使用 `TriggerJobRequest.broadcastApi`，通过 `broadcastSelector` 限定 tag `manual-demo` 和 label `worker_pool=boot3-blue`。
6. 失败与边界：触发 `demo.fail`，确认 instance 日志和失败状态；禁用脚本或移除 runtime 后确认不可用 runner 没有出现在 structured capabilities。不要把 `/demo/*` 运维端点当成 Worker inbound dispatch 面。

## 前置条件

执行本页命令前，请先满足页面列出的安装、认证和权限要求。本地示例默认 Server 使用 `config/dev.toml`，客户端访问 `127.0.0.1`，令牌保存在 shell 变量中，不写入文件或截图。

## 验收

完成本页步骤后，用对应 API、UI、构建、smoke 或部署检查验证结果。有效验收至少包含执行的命令、检查的路由或文件，以及观察到的状态或产物。

## 故障排查

步骤失败时，先保留完整命令、响应状态和 Server 日志时间窗口，再检查认证、namespace/app scope、Worker 匹配、存储 readiness 和代理行为，不要直接修改生产配置。

## 生产检查清单

- [ ] 密钥通过环境变量或平台 Secret 引用管理，不写入示例。
- [ ] 已把本地 `127.0.0.1` 命令替换成真实域名、TLS 和认证方式。
- [ ] 已记录变更面的回滚和证据采集方式。
- [ ] 运维人员可以在没有隐藏 shell 历史或隐式状态的情况下复现验收。
