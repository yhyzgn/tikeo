---
title: Java Spring Boot Starter
description: Java SDK, Spring adapter, and Spring Boot starter compatibility docs.
---

# Java Spring Boot Starter

The Java SDK is a Gradle multi-module SDK with separate Spring Framework adapters and Spring Boot starter compatibility lines.

## Verify the SDK

```bash
cd sdks/java
./gradlew test --no-daemon
./gradlew jar sourcesJar --no-daemon
```

## Verify demos

```bash
cd examples/java/spring-boot2-worker-demo && ./gradlew test --no-daemon
cd examples/java/spring-boot3-worker-demo && ./gradlew test --no-daemon
cd examples/java/spring-boot4-worker-demo && ./gradlew test --no-daemon
```

## Compatibility rule

Java modules must keep explicit source/resource/test boundaries. Do not replace compatibility modules with empty source-set indirection.
