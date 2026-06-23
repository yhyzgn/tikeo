---
title: Java SDK and Spring Boot Starter
description: Java/Spring 依赖、最小 Worker、异常处理和 Management client 写法。
---

# Java SDK and Spring Boot Starter

先读 [SDK 与 API 集成指南](../integrations/sdk-and-api)。本文只说明 Java/Spring 特有的依赖安装、最小 Worker、异常捕获和 Management client 写法。Java SDK 位于 `sdks/java`；demo 位于 `examples/java/spring-boot{2,3,4}-worker-demo`。

## 前置条件

| Requirement | Value |
| --- | --- |
| Group | `net.tikeo` |
| 版本占位符 | `${TIKEO_VERSION}`，来自 README 顶部包徽标或 release tag；仓库开发版本在 `sdks/java/gradle.properties` 的 `tikeoVersion` 中维护，发布时由 tag 同步。 |
| Java release | `17` |
| Core module | `tikeo` |
| Spring modules | `tikeo-spring`, `tikeo-spring6`, `tikeo-spring5` |
| Boot starters | `tikeo-spring-boot-starter`, `tikeo-spring-boot3-starter`, `tikeo-spring-boot2-starter` |

Spring Boot 3 的 Gradle 依赖示例：

```kotlin
dependencies {
    implementation("net.tikeo:tikeo-spring-boot3-starter:${TIKEO_VERSION}")
}
```

验证仓库内 SDK：

```bash
./sdks/java/gradlew -p sdks/java test --no-daemon
./sdks/java/gradlew -p sdks/java :tikeo:test --no-daemon
```

## 最小 Worker

Spring Boot starter 从 `tikeo.worker.*` 绑定 Worker 配置。默认启用 Worker auto-configuration 和 auto-startup；部署时必须显式设置 namespace/app 和 endpoint。Tikeo Server / Worker Tunnel 临时不可达时，starter 只记录 warning，不阻塞业务应用启动，worker client 会继续后台重连。

```yaml
tikeo:
  worker:
    enabled: true
    endpoint: ${TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}
    namespace: ${TIKEO_WORKER_NAMESPACE:sdk-smoke}
    app: ${TIKEO_WORKER_APP:management}
    client-instance-id: ${TIKEO_WORKER_CLIENT_INSTANCE_ID:java-worker-1}
    cluster: ${TIKEO_WORKER_CLUSTER:local}
    region: ${TIKEO_WORKER_REGION:local}
    labels:
      worker_pool: java-blue
  management:
    enabled: true
    endpoint: ${TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9090}
    api-key: ${TIKEO_MANAGEMENT_API_KEY:}
    namespace: ${TIKEO_WORKER_NAMESPACE:sdk-smoke}
    app: ${TIKEO_WORKER_APP:management}
```

最小 processor：

```java
package demo;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TikeoProcessor;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

@Component
class EchoProcessor {
    private static final Logger log = LoggerFactory.getLogger(EchoProcessor.class);

    @TikeoProcessor("demo.echo")
    TaskOutcome echo(TaskContext task) {
        log.info("java echo processor={} instance={}", task.processorName(), task.instanceId());
        return TaskOutcome.success("java echo processed");
    }
}
```

`@TikeoProcessor(value = "billing.sql-sync", kind = TikeoProcessorKind.PLUGIN, pluginType = "sql")` 只用于已实现 plugin processor。普通 SDK processor 不应使用 `script:*` 名称。

## 异常捕获

| Case | Java/Spring 行为 |
| --- | --- |
| 预期业务失败 | 返回 failure `TaskOutcome`。 |
| Processor exception | 抛异常；adapter 上报 task failure，message 进入任务证据。 |
| 不支持的 processor | 不注册 annotation，或从显式 handler 返回 failure outcome。 |
| Task logs | 优先使用 SLF4J/Logback + `TikeoTaskLogbackAppender`；`TaskContext.logInfo/logError` 仅作为 fallback。 |

## Management client 写法

核心 Java helper 是 `HttpTikeoJobClient`；接口是 `TikeoJobClient`。

```java
import net.tikeo.management.client.HttpTikeoJobClient;
import net.tikeo.management.client.TikeoJobClient;
import net.tikeo.management.model.BroadcastSelectorRequest;
import net.tikeo.management.model.CreateJobRequest;
import net.tikeo.management.model.TriggerJobRequest;
import java.util.Map;

class ManagementExample {
    void run() {
        TikeoJobClient client = new HttpTikeoJobClient(
                "http://127.0.0.1:9090",
                System.getenv("TIKEO_MANAGEMENT_API_KEY"),
                "sdk-smoke",
                "management");
        var job = client.createJob(CreateJobRequest.api("java-echo-api", "demo.echo"));
        var instance = client.triggerJob(job.id(), TriggerJobRequest.api());
        var broadcast = TriggerJobRequest.broadcastApi(new BroadcastSelectorRequest(null, null, null, Map.of("worker_pool", "java-blue")));
        System.out.printf("instance=%s triggerType=api executionMode=single%n", instance.id());
        System.out.printf("broadcastSelector=%s%n", broadcast.broadcastSelector());
    }
}
```

`HttpTikeoJobClient(endpoint, apiKey, namespace, app)` 发送 `x-tikeo-api-key`，trim endpoint，空 namespace/app 默认 `default`，并提供 `CreateJobRequest.api`、`TriggerJobRequest.api`、`TriggerJobRequest.broadcastApi`、`BroadcastSelectorRequest`：

- Create helper → [`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- Trigger helper → [`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- Instance polling → [`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- Log inspection → [`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker dispatch → [`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## 验收

| Check | Command or evidence |
| --- | --- |
| SDK tests | `./sdks/java/gradlew -p sdks/java test --no-daemon` |
| Worker registration | Worker 带 `demo.echo` 和 `worker_pool=java-blue`。 |
| API trigger | Instance 显示 `triggerType=api` 和 `executionMode=single`。 |
| Worker logs | Instance logs 包含 `java echo processed` 或 processor-specific log。 |

## 故障排查

| 现象 | 修复 |
| --- | --- |
| Starter 不连接 | 检查 `tikeo.worker.enabled`、`auto-startup`、endpoint 和 dry-run。 |
| Management bean unauthorized | 确认 `tikeo.management.api-key` 来自 `TIKEO_MANAGEMENT_API_KEY` 并发送 `x-tikeo-api-key`。 |
| Annotation 未发现 | 确保 processor class 是 component scan 中的 Spring bean。 |
| Broadcast selector 未生效 | 使用 `TriggerJobRequest.broadcastApi` 和 `BroadcastSelectorRequest`；默认 trigger 仍是 single。 |

## 生产检查清单

- [ ] 选择匹配 Spring Boot major version 的 starter。
- [ ] 部署环境不要依赖本地 endpoint 默认值。
- [ ] API key 从 secret 注入，不写进 `application.yml` plaintext。
- [ ] Processor exception 对 operator 可见。
- [ ] Broadcast selector 包含足够 labels/tags，避免意外 fan-out。


## 统一配置参数与默认值

不同语言 SDK 的代码写法不同，但接入 Tikeo 时面对的是同一组语义。不要把这些参数理解成各语言私有字段；它们最终都会进入 Worker Tunnel 注册、任务派发、Management API 创建任务和实例触发链路。

| 参数 | 默认值 | 生产建议 |
| --- | --- | --- |
| `endpoint` | 本地 Worker Tunnel 通常是 `http://127.0.0.1:9998` | 生产必须指向 Server 暴露的 Worker Tunnel 地址，并与 TLS/mTLS 配置一致。 |
| `namespace` | `default` 或示例中的 `sdk-smoke` | 每个团队、租户或环境应使用清晰命名，不要把生产任务混进 default。 |
| `app` | `default` 或示例中的 `management` | 与 Management API Key 的 app scope 保持一致。 |
| `clientInstanceId` | 示例手工指定 | 生产中应唯一且稳定，便于 Worker 页面和审计定位。 |
| `cluster` / `region` | `local` | 多机房部署必须真实填写，广播和选择器会使用这些信息。 |
| `labels` | 空 map | 用 `worker_pool`、`region`、`cluster` 等标签表达调度边界。 |
| `sdkProcessors` | 空列表 | 只声明当前进程真实实现的 processor，避免实例被派发后失败。 |
| `heartbeat` | 约 10 秒 | 保持默认即可；高延迟网络再根据运维策略调整。 |

## 管理客户端凭证

Management client 使用应用级 API Key，不使用浏览器里的人工登录 token。创建 key 时要绑定 namespace/app，运行时通过 `TIKEO_MANAGEMENT_API_KEY` 注入。所有语言的请求都会发送 `x-tikeo-api-key`，创建任务时应明确 `triggerType=api`、`executionMode=single`，需要广播时再设置 `broadcastSelector`。

| 决策 | 推荐做法 | 风险 |
| --- | --- | --- |
| API Key 保存位置 | Secret Manager、Kubernetes Secret 或 CI secret | 不要写进代码、README、截图或 shell 历史。 |
| 权限范围 | app-scoped service account | 不要用 Owner 或全局管理账号跑 SDK。 |
| 轮换 | 发布窗口内双写新旧 key | 直接删除旧 key 会让 Worker 或自动化立即失败。 |
| 验证 | 先创建 API 手动触发任务，再触发一次 | 只构建通过不能证明 Management API 可用。 |

## 现场验收 runbook

1. 确认 Server `/readyz` 通过，Web 控制台能看到目标 namespace/app。
2. 使用当前语言启动一个只声明 `demo.echo` 的 Worker。
3. 在 Worker 页面确认 `clientInstanceId`、region、cluster、labels 和 processor 列表正确。
4. 使用 Management client 创建 API 触发任务，确认返回 job id。
5. 触发一次 single instance，进入 Instances 页面查看状态、Worker、日志和 result。
6. 如果要验证广播，设置 `broadcastSelector`，确认多个符合标签的 Worker 都生成 attempt 或广播实例证据。
7. 制造一次业务失败和一次运行时异常，确认日志中能看到 message、stack 或错误路径。
8. 给失败事件绑定通知渠道，确认消息中的实例 ID、时间、状态、操作人、执行类型可以追溯。

## 故障排查表

| 现象 | 可能原因 | 处理方式 |
| --- | --- | --- |
| Worker 页面看不到进程 | endpoint/TLS/mTLS 或 token 不匹配 | 先看 Worker 启动日志，再看 Server Worker Tunnel 日志。 |
| 实例一直等待 | processorName、标签或 region/cluster 不匹配 | 对照 Jobs 页和 Workers 页的 capability。 |
| 触发 API 返回 401/403 | `TIKEO_MANAGEMENT_API_KEY` 无效或 scope 不对 | 重新创建 app-scoped key，确认 header 是 `x-tikeo-api-key`。 |
| 执行失败但没有日志 | processor 异常未被 SDK 捕获或进程崩溃 | 升级 SDK，确保 task log API 被调用，并查看 Worker 本地日志。 |
| 广播没有命中目标 | `broadcastSelector` 标签与 Worker labels 不一致 | 先用单实例验证，再逐步加 selector。 |

## 生产检查清单

- [ ] 依赖坐标固定到发布版本，而不是随意使用本地源码路径。
- [ ] WorkerConfig 默认值已经被生产环境显式覆盖。
- [ ] 最小 Worker 在目标环境成功注册并展示能力。
- [ ] 管理客户端凭证来自 Secret，不来自人工账号。
- [ ] 现场验收 runbook 的创建、触发、日志、失败、通知链路均通过。


### Spring Boot 属性默认值

本节保留该关键术语，确保中文文档与英文 operator-grade SDK 指南保持同等检索深度。Java/Spring 模块位于 `sdks/java`，不同 Boot 版本使用不同 starter。
