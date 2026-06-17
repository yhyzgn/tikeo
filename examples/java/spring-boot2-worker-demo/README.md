# Spring Boot 2 Worker demo ☕

[🇨🇳 中文示例文档](../../../README.zh-CN.md#能证明产品价值的快速开始)

This demo validates the Tikeo Java SDK and the Spring Boot 2.x starter line with `tikeo-spring-boot2-starter`.

## Run

```bash
cd examples/java/spring-boot2-worker-demo
./gradlew bootRun
```

Prefer dry-run mode for local startup checks without a Tikeo server:

```bash
TIKEO_WORKER_DRY_RUN=true ./gradlew bootRun
```

## Management API create + trigger example

Enable `tikeo.management.enabled=true` and configure an app-scoped API key to expose demo endpoints
that create jobs and immediately trigger them through the Java management SDK:

```bash
TIKEO_MANAGEMENT_ENABLED=true \
TIKEO_MANAGEMENT_ENDPOINT=http://127.0.0.1:9090 \
TIKEO_MANAGEMENT_API_KEY=<app-scoped-sdk-key> \
./gradlew bootRun

# In another shell, create + trigger examples:
curl -X POST http://127.0.0.1:18082/demo/jobs/echo
curl -X POST http://127.0.0.1:18082/demo/jobs/plugin/sql
curl -X POST http://127.0.0.1:18082/demo/jobs/script/script_manual_shell_echo
```

Each endpoint calls `createJob(...)` and then `triggerJob(..., TriggerJobRequest.api())`, returning
the created job plus the triggered instance with `triggerType=api` and `executionMode=single`.

## Verify

```bash
./gradlew clean test --no-daemon
```

## Operational cautions

- Use the matching starter artifact for the Spring Boot major version.
- Configure SLF4J/Logback or Log4j2 in the application for console plus file diagnostics.
- Task logs must be emitted through `TaskContext`, not by capturing unrelated process output.
- Keep sandbox runtime checks and auto-install enabled for manual acceptance unless testing failure modes.
