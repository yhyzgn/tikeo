---
title: 准备演示数据
description: 用安全、可追溯的方式为 Tikeo 本地评估准备 demo 数据。
---

# 准备演示数据

演示数据的目标不是把看板塞满，而是展示 Tikeo 的真实产品优势：租户与 app 隔离、Worker 能力匹配、工作流回放、脚本治理、告警投递和审计证据。公开文档只应推荐有真实命令、测试或录屏证据支撑的路径。

## 当前安全路径

- 使用仓库内 `config/dev.toml` 启动本地 Server。
- 使用 `examples/rust`、`examples/go`、`examples/java` 下已验证的 Worker demo。
- 使用宣传录屏作为视觉证据，但不要把它当成自动化验收测试。

## 通过 HTTP 创建简单任务

在本地开发会话完成认证后，可以通过 typed API 创建任务。具体 payload 取决于当前启用的认证与 scope 配置：

```bash
curl -fsS http://0.0.0.0:9090/api/v1/jobs \
  -H 'content-type: application/json' \
  -d '{"namespace":"default","app":"demo","name":"manual-demo"}'
```

如果启用了授权，请带上本地配置要求的 session 或 API-key header。

## 不建议的方式

不要为了公开 demo 手工插入数据库行。手工行往往绕开 RBAC、审计、migration 和领域校验，最终会让 Web、API、审计日志之间出现不一致。

## 推荐演示叙事

1. 创建一个 namespace/app 作为演示 scope。
2. 连接 Rust 或 Go Worker，并展示明确 processor capability。
3. 创建 API 触发任务并运行。
4. 查看 instance attempt、日志和结果。
5. 创建一个小型工作流并展示节点关系。
6. 展示 Worker session history 与审计记录。
7. 触发一个故意的策略拒绝或能力缺失场景，解释失败原因如何可见。

## 数据质量规则

演示数据必须有真实关系和可追溯证据：任务属于 scope，实例来自触发，日志来自 attempt，Worker 能力来自实际 runtime。不要用随机行制造“繁忙感”。
