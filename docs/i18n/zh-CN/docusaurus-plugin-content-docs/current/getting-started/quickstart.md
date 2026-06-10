---
title: 快速开始：Server + Web + Worker
description: 本地启动 Tikeo、打开 Web 控制台，并连接一个已验证 Worker demo。
---

# 快速开始：Server + Web + Worker

本页提供最短的真实评估路径：启动 Server，检查健康状态，启动 Web 控制台，再连接一个 Worker demo。不要只验证首页能打开；Tikeo 的关键价值在于 Worker 主动出站连接、任务派发、日志、结果和审计证据形成闭环。

## 1. 启动 Server

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

检查 HTTP 健康状态：

```bash
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/readyz
```

默认开发配置下，HTTP API 使用 `9090`，Worker Tunnel 使用 `9998`。

## 2. 启动 Web 控制台

```bash
cd web
bun install --frozen-lockfile
bun run dev
```

Web 控制台用于查看 Dashboard、Jobs、Instances、Workflows、Workers、Scripts、Alerts、Audit、Settings 等运营视图。评估时建议同时观察 Web 和 API 输出，确保界面证据与后端状态一致。

## 3. 连接 Worker demo

Rust Worker demo：

```bash
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Go、Java、Python 与 Node.js demo 也在仓库中提供独立验证入口。公开文档只应宣传真实可运行、可验证的 demo 路径，不要把未来能力写成当前能力。

## 4. 观察闭环

一个完整的快速开始应至少确认：Worker 注册成功、能力快照可见、任务能够被派发、实例日志有结果、失败原因可被审计。只看到进程启动不代表调度链路已经验证。

## 5. 下一步

- 想理解架构边界，阅读 [Worker Tunnel](../concepts/worker-tunnel)。
- 想准备演示数据，阅读 [Seed demo data](./seed-demo-data)。
- 想部署到容器环境，阅读 [Docker Compose](../deployment/docker-compose)。
