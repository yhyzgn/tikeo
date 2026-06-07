# Java examples ☕

[🇨🇳 中文示例文档](../../README.zh-CN.md#能证明产品价值的快速开始)

Java examples are split by Spring Boot major version so each demo validates the matching starter
artifact and dependency constraints.

| Demo | Spring Boot | Starter | Verification |
| --- | --- | --- | --- |
| `spring-boot2-worker-demo` | 2.7.x | `tikeo-spring-boot2-starter` | `./gradlew clean test --no-daemon` |
| `spring-boot3-worker-demo` | 3.5.x | `tikeo-spring-boot3-starter` | `./gradlew clean test --no-daemon` |
| `spring-boot4-worker-demo` | 4.x | `tikeo-spring-boot-starter` | `./gradlew clean test --no-daemon` |

All demos use structured worker capabilities, precise task logs, SLF4J diagnostics, stable
client-instance identity, and optional management API examples.

Operational cautions: `TIKEO_WORKER_DRY_RUN=true` prevents live registration and is suitable only for
local startup checks. For manual acceptance, use the demo scripts so namespace/app/worker-pool values
match seeded jobs.
