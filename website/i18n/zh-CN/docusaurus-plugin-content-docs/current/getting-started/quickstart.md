---
title: 快速开始：Server + Web + Worker
description: 本地启动 Tikeo 并连接一个已验证的 Worker demo。
---

# 快速开始：Server + Web + Worker

## 1. 启动 Server

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

检查：

```bash
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/readyz
```

默认开发形态下，HTTP API 使用 `9090`，Worker Tunnel 使用 `9998`。

## 2. 启动 Web 控制台

```bash
cd web
bun install --frozen-lockfile
bun run dev
```

## 3. 连接 Worker demo

Rust Worker demo：

```bash
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Go 和 Java demo 也在仓库 CI 中持续验证。公开文档只应宣传真实可运行、可验证的 demo 路径。
