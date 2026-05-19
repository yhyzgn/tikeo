# 005-worker-sdk-rust-and-java-starter-planning：Worker SDK 基础与 Java Starter 规划

> 本阶段提示词需在 Worker Tunnel 与基础调度链路完成后更新。

## 目标

- 实现 Rust Worker SDK 的最小可用能力。
- 规划并初始化 Java SDK 目录结构，Java 端优先支持 Spring Boot Starter 模式。

## Java SDK 硬性约束

- Java SDK 优先提供 `scheduler-spring-boot-starter`。
- 业务侧应通过 Spring Boot auto-configuration 和 `@SchedulerProcessor` 注解接入。
- Java Worker 必须主动连接 scheduler Worker Tunnel，不得要求业务应用暴露入站端口。
- 需要规划 `scheduler-java-core`、`scheduler-spring-boot-autoconfigure`、`scheduler-spring-boot-starter`。

## 验证

Rust 侧仍需执行完整 cargo 验证。Java 工程初始化后需补充 Gradle/Maven 验证命令。
