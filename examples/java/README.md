# Java examples

The Java examples are split by Spring Boot major version so each demo validates the matching starter artifact in an independent Gradle project/work directory.

| Demo | Spring Boot | tikee starter | Default port | Verification |
| --- | --- | --- | --- | --- |
| `spring-boot2-worker-demo` | 2.7.x | `tikee-spring-boot2-starter` | `18082` | `(cd examples/java/spring-boot2-worker-demo && ./gradlew clean test --no-daemon)` |
| `spring-boot3-worker-demo` | 3.5.x | `tikee-spring-boot3-starter` | `18083` | `(cd examples/java/spring-boot3-worker-demo && ./gradlew clean test --no-daemon)` |
| `spring-boot4-worker-demo` | 4.0.x | `tikee-spring-boot-starter` | `18084` | `(cd examples/java/spring-boot4-worker-demo && ./gradlew clean test --no-daemon)` |

Each demo contains its own `settings.gradle.kts`, `build.gradle.kts`, Gradle wrapper, `src/main`, `src/test`, and README. They all include the same worker use-case surface: task processors, worker lifecycle/logging, processor registry invocation, management API examples, script/API/plugin job management examples, and starter compatibility assertions.

## Single-server multi-worker integration data

Use the development integration seed and worker matrix scripts when validating one tikee server with multiple Java demo workers in different tenant scopes.

1. Start the server/web stack from the repository root:

   ```bash
   ./scripts/dev.sh
   ```

2. Seed tenant scopes, worker pools, and API-triggered demo jobs through the management API:

   ```bash
   scripts/dev-integration-seed.sh
   ```

3. Start all Java demo workers against the same server:

   ```bash
   scripts/start-java-demo-workers.sh
   ```

   Use `--detach` to leave them running in the background, `--status` to inspect PIDs/logs, and `--stop` to stop the matrix.

| Worker id | Demo | Scope | Port |
| --- | --- | --- | --- |
| `java-boot2-orders-blue` | Spring Boot 2 | `dev-alpha/orders/boot2-blue` | `18182` |
| `java-boot3-orders-blue` | Spring Boot 3 | `dev-alpha/orders/boot3-blue` | `18183` |
| `java-boot4-billing-green` | Spring Boot 4 | `dev-alpha/billing/boot4-green` | `18184` |
| `java-boot3-analytics-batch` | Spring Boot 3 | `dev-beta/analytics/boot3-batch` | `18185` |
| `java-boot4-ops` | Spring Boot 4 | `dev-ops/automation/boot4-ops` | `18186` |

The demo applications read `TIKEE_WORKER_NAMESPACE`, `TIKEE_WORKER_APP`, and `TIKEE_WORKER_POOL` from the environment. `worker_pool` is also advertised as a worker label so the server worker list can bind each worker to the seeded pool.
