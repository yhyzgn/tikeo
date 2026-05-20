# 017-alerting-and-observability

## 背景

016-dynamic-script-sandbox 已完成脚本管理基础切片：
- 后端具备脚本定义 CRUD API（`/api/v1/scripts`），Admin 权限保护。
- Web 管理端已有脚本管理页面（列表、创建、审批、启用/禁用、删除）。
- Worker 侧沙箱执行器尚未实现，留待后续阶段落地。
- 脚本策略引擎尚未实现。

当前系统缺少可观测性（无指标暴露、无结构化审计日志）和告警能力。这两个是企业级部署的基础前置条件。

## 目标

为 scheduler 添加 Prometheus 指标暴露、基础审计日志持久化与查询、简单告警通知（Webhook / 邮件）。

## 关键约束

- Prometheus 指标使用 `metrics` + `metrics-exporter-prometheus` crate，HTTP 端口复用或独立暴露。
- 审计日志持久化到 DB（新增 `audit_logs` 表），HTTP API 查询，Admin 权限保护。
- 告警通知支持 Webhook（必选）和 SMTP 邮件（可选，配置启用）。
- 告警触发条件：Job 执行失败超阈值、Worker 离线、调度延迟超阈值。
- Rust workspace 保持 `crates/*` 模块解耦；根主程序入口仍为 `src/main.rs`。
- Web 保持 `web/` + React + Ant Design + Bun。
- 禁止 Swagger UI，仅保留 `/api-docs/openapi.json`。
- 数据库禁止外键，只允许字段软关联。

## 建议范围

1. Storage：新增 `audit_logs` 表（actor/action/resource_type/resource_id/detail/ip_address/created_at），无外键。
2. HTTP API：审计日志写入（内部调用，非直接暴露）、`GET /api/v1/audit-logs` 查询（Admin 权限保护）。
3. 指标：集成 `metrics` facade，在 HTTP middleware 和 Worker Tunnel 关键路径埋点，暴露 `/metrics` Prometheus 端点。
4. 告警：定义告警规则模型，实现 Webhook 通知通道，预留邮件通道接口。
5. Web：审计日志查询页面，Prometheus 指标概览（简单卡片展示，非 Grafana 替代）。
6. 测试：审计日志写入/查询、指标计数器、告警触发。

## 验证要求

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
```

完成后更新 `design/scheduler-architecture-design.md`、`.memory/*`、后续 `.prompt/018-*.md`，提交并推送。
