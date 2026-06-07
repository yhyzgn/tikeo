# Spring Boot 4 Worker demo ☕

[🇨🇳 中文示例文档](../../../docs/zh-CN/examples.md)

This demo validates the Tikeo Java SDK and the Spring Boot 4 starter line.

## Run

```bash
cd examples/java/spring-boot4-worker-demo
./gradlew bootRun
```

Prefer dry-run mode for local startup checks without a Tikeo server:

```bash
TIKEO_WORKER_DRY_RUN=true ./gradlew bootRun
```

## Verify

```bash
./gradlew clean test --no-daemon
```

## Operational cautions

- Use the matching starter artifact for the Spring Boot major version.
- Configure SLF4J/Logback or Log4j2 in the application for console plus file diagnostics.
- Task logs must be emitted through `TaskContext`, not by capturing unrelated process output.
- Keep sandbox runtime checks and auto-install enabled for manual acceptance unless testing failure modes.
