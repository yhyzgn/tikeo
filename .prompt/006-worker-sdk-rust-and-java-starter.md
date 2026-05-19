# 006-worker-sdk-rust-and-java-starter：Worker SDK 基础与 Java Starter 规划

> 本阶段应在 Worker Tunnel、Jobs 存储和基础 API 触发实例链路完成后执行。

## 目标

- 实现 Rust Worker SDK 的最小可用能力：主动连接 Worker Tunnel、注册、心跳、基础任务处理器接口。
- 规划并初始化 Java SDK 目录结构，Java 端优先支持 Spring Boot Starter 模式。

## 当前上下文

- Worker 必须主动出站连接 scheduler，不要求业务应用暴露入站端口。
- Worker Tunnel 当前默认监听 `127.0.0.1:9091`。
- HTTP 默认监听 `127.0.0.1:9090`。
- 存储层已使用 SeaORM 1.1.20 稳定线，支持 SQLite dev DB 和 MySQL feature-enabled migration。

## Java SDK 硬性约束

- Java SDK 优先提供 `scheduler-spring-boot-starter`。
- 业务侧应通过 Spring Boot auto-configuration 和 `@SchedulerProcessor` 注解接入。
- Java Worker 必须主动连接 scheduler Worker Tunnel，不得要求业务应用暴露入站端口。
- 需要规划 `scheduler-java-core`、`scheduler-spring-boot-autoconfigure`、`scheduler-spring-boot-starter`。

## 验证

Rust 侧仍需执行完整 cargo 验证。Java 工程初始化后需补充 Gradle/Maven 验证命令。
