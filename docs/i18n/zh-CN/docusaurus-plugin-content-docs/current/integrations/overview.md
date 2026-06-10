---
title: 集成概览
description: OpenAPI、gRPC、Prometheus、OpenTelemetry、OIDC、Terraform、Kubernetes 与告警渠道集成地图。
---

# 集成概览

Tikeo 的集成按运营边界分组，而不是把所有外部系统混成一张能力清单。

| 集成 | 目的 |
|---|---|
| HTTP API / OpenAPI | 管理 API 与 Web 控制台契约 |
| gRPC / protobuf | Worker Tunnel 协议 |
| Prometheus / Grafana | 指标、SLO 与 dashboard 模板 |
| OpenTelemetry | trace export 与请求关联 |
| OIDC | 外部身份映射到本地 Tikeo session |
| Terraform Provider | GitOps/IaC manifest 与 drift workflow |
| Kubernetes Operator | Tikeo manifest reconcile 与状态证据 |
| Alert channels | Webhook、email、Slack、钉钉、飞书、企微、PagerDuty |

## 设计原则

每个集成都必须保留 Tikeo 的权威边界。Server 拥有调度、治理、状态、API 和审计；Worker 拥有执行；部署集成负责打包与 reconcile；观测集成报告证据，但不应成为任务状态事实源。

## 当前高价值路径

- OpenAPI / HTTP API：支撑 Web 与 SDK management client。
- gRPC / protobuf：支撑 Worker Tunnel 协议。
- Prometheus / Grafana：支撑 SLO 与运行时可见性。
- OpenTelemetry：支撑请求链路和 trace export。
- OIDC：把企业身份映射到本地用户、角色和 scope。
- Terraform Provider / Kubernetes Operator：支撑 GitOps 与 IaC。
- Alert providers：支撑重试、DLQ 和通知证据。

## 验证优先级

优先补能在本地或 CI 证明的集成。一个有用的集成页面应该说明 owner component、source artifact、验证命令，以及 credentials、schema 或网络路径错误时应出现的失败证据。

## 文档规则

Reference 页面最终应由 OpenAPI、protobuf 或 source artifact 生成。在生成链路接入前，集成文档保持概念性，链接到已验证命令，避免手工编造 schema。
