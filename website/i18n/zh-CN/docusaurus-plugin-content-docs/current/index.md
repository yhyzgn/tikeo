---
title: Tikeo 是什么？
slug: /
description: Tikeo 是面向任务、工作流、Worker Tunnel、多语言 Worker 与受治理脚本的 Rust 原生编排平台。
---

# Tikeo 是什么？

Tikeo 是用 Rust 构建的分布式任务调度与计算编排平台。它把 Server、Web 控制台、主动出站连接的 Worker、工作流 DAG、多语言 SDK、脚本治理、RBAC、审计日志与部署资产整合到同一个项目中。

## 核心差异

Worker 通过 gRPC/HTTP2 Worker Tunnel 主动连接 Server。Server 复用这条长连接完成调度、取消、心跳、日志和结果回传，因此业务 Worker 不需要暴露入站执行端口。

## 快速评估路径

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
(cd examples/rust/worker-demo && cargo run)
```

下一步阅读：[快速开始](./getting-started/quickstart)。
