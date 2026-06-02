# Java examples

The Java examples are split by Spring Boot major version so each demo validates the matching starter artifact in an independent Gradle project/work directory.

| Demo | Spring Boot | tikee starter | Default port | Verification |
| --- | --- | --- | --- | --- |
| `spring-boot2-worker-demo` | 2.7.x | `tikee-spring-boot2-starter` | `18082` | `(cd examples/java/spring-boot2-worker-demo && ./gradlew clean test --no-daemon)` |
| `spring-boot3-worker-demo` | 3.5.x | `tikee-spring-boot3-starter` | `18083` | `(cd examples/java/spring-boot3-worker-demo && ./gradlew clean test --no-daemon)` |
| `spring-boot4-worker-demo` | 4.0.x | `tikee-spring-boot-starter` | `18084` | `(cd examples/java/spring-boot4-worker-demo && ./gradlew clean test --no-daemon)` |

Each demo contains its own `settings.gradle.kts`, `build.gradle.kts`, Gradle wrapper, `src/main`, `src/test`, and README. They all include the same worker use-case surface: task processors, worker lifecycle/logging, processor registry invocation, management API examples, script/API/plugin job management examples, and starter compatibility assertions.

`spring-worker-demo` is kept as the original Boot 3 demo path for continuity, but new version-specific validation should use the three `spring-boot*-worker-demo` directories above.
