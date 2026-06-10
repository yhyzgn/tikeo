---
title: Tikeo 是什么？
slug: /
description: Tikeo 是面向任务、工作流、Worker Tunnel、多语言 Worker 与受治理脚本的 Rust 原生编排平台。
---

# Tikeo 是什么？

Tikeo 是用 Rust 构建的分布式任务调度与计算编排平台。它把 Server、Web 控制台、主动出站连接的 Worker、工作流 DAG、多语言 SDK、脚本治理、RBAC、审计日志与部署资产整合到同一个项目中，目标是成为企业内部统一调度底座，而不是另一个只能跑定时任务的小工具。

## 为什么值得评估

许多传统调度系统默认中心服务可以直接回连业务执行器。这个模型在 Kubernetes 私有 namespace、跨 VPC、NAT、服务网格、企业防火墙或多集群场景下很容易失败。Tikeo 反转边界：Worker 通过 gRPC/HTTP2 Worker Tunnel 主动连接 Server，Server 复用这条长连接完成派发、取消、心跳、日志和结果回传。

## 首次评估路径

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
(cd examples/rust/worker-demo && cargo run)
```

建议先验证 Server 和 Web，再启动 Rust、Go、Java、Python 或 Node.js Worker demo。这样可以确认评估覆盖了真正的 Worker Tunnel，而不仅仅是 HTTP API 能否响应。

## 当前实现范围

仓库当前包含 Rust Server、Web UI、Worker Tunnel、持久化 Worker 会话可见性、任务/实例/调度/attempt/log、工作流、脚本治理、告警、RBAC、OIDC、指标、审计和部署资产。Rust、Go、Java、Python、Node.js SDK/demo surface 已纳入 CI 或本地验证路径；文档只应宣传可以从仓库证据中追溯的能力。

## 适合关注的产品优势

Tikeo 最适合评估复杂运行环境：Worker 不能暴露入站端口、执行历史必须可审计、多语言团队需要统一协议、脚本需要审批和沙箱、工作流需要拓扑与回放、部署需要同时覆盖 Docker Compose 与 Kubernetes/Helm。

## 阅读路线

- [安装要求](./getting-started/installation)
- [快速开始](./getting-started/quickstart)
- [Worker Tunnel](./concepts/worker-tunnel)
- [Kubernetes 与 Helm](./deployment/kubernetes)
