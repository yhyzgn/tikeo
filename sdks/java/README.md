# Tikeo Java SDKs ☕

[🇨🇳 中文 SDK 文档](../../docs/zh-CN/sdk.md)

Java SDK modules are published as independent Gradle subprojects. Each module owns its dependencies,
tests, sources JAR, and Maven publication so Spring Boot 2/3/4 compatibility stays explicit.

| Module | Purpose |
| --- | --- |
| `tikeo` | Native Java Worker Tunnel client, task contracts, sandbox tools, and management API client. |
| `tikeo-spring` | Spring Framework 7 adapter for the primary Spring Boot 4 starter. |
| `tikeo-spring5` | Spring Framework 5 adapter for Spring Boot 2.x projects. |
| `tikeo-spring6` | Spring Framework 6 adapter for Spring Boot 3.x projects. |
| `tikeo-spring-boot-starter` | primary Spring Boot 4.x starter. |
| `tikeo-spring-boot2-starter` | Spring Boot 2.x projects compatibility starter. |
| `tikeo-spring-boot3-starter` | Spring Boot 3.x projects compatibility starter. |

## Usage

```yaml
tikeo:
  worker:
    enabled: true
    auto-startup: true
    endpoint: http://127.0.0.1:9998
    namespace: dev-alpha
    app: orders
    worker-pool: java-green
```

Annotated processors are discovered by the Spring adapters:

```java
@TikeoProcessor(value = "billing.reconcile")
public TaskOutcome reconcile(TaskContext context) {
    context.logInfo("billing reconcile started");
    return TaskOutcome.succeeded();
}
```

## Logging

The Java SDK uses SLF4J for SDK diagnostics and `TaskContext` for task instance logs. Configure the
host application logging backend for console plus file output, for example with Logback:

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

## Verification

```bash
(cd sdks/java && ./gradlew clean test publishToMavenLocal --no-daemon)
```
