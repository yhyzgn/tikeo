# 018-alerting-and-observability

## 背景

017-script-versioning-and-diff 已完成脚本版本历史、diff 对比、编辑器和完整审批流程。
当前系统缺少可观测性（无指标暴露、无结构化审计日志）和告警能力。

## 目标

为 tikeo 添加 Prometheus 指标暴露、审计日志持久化与查询、基础告警通知（Webhook）。

## 关键约束

- Prometheus 指标使用 `metrics` + `metrics-exporter-prometheus` crate，在主 HTTP 服务上暴露 `/metrics`。
- 审计日志持久化到 DB（新增 `audit_logs` 表），HTTP API 查询，Admin 权限保护。
- 告警通知支持 Webhook（必选），预留邮件通道接口。
- 告警触发条件：Job 执行失败超阈值、Worker 离线、调度延迟超阈值。
- Rust workspace 保持 `crates/*` 模块解耦；根主程序入口仍为 `src/main.rs`。
- Web 保持 `web/` + React + Ant Design + Bun。
- 禁止 Swagger UI，仅保留 `/api-docs/openapi.json`。
- 数据库禁止外键，只允许字段软关联。
- 所有 HTTP 接口继续返回 `{code,message,data}` envelope。

## 建议范围

1. Storage：新增 `audit_logs` 表（id/actor/action/resource_type/resource_id/detail/ip_address/created_at），无外键。migration + SQLite compat。
2. Repository：`AuditLogRepository`，`append()` + `list(filters)` + `get(id)`。
3. HTTP API：`GET /api/v1/audit-logs`（Admin 权限，分页查询）。OpenAPI schema。
4. 审计埋点：在关键写操作（create/update/delete job, trigger, script CRUD, user CRUD, login/logout）中调用 `audit.append()`。
5. 指标：集成 `metrics` facade，HTTP middleware 记录请求计数/延迟，Worker Tunnel 记录连接数/任务数，暴露 `GET /metrics`。
6. 告警：定义 `AlertRule` 模型，实现 Webhook 通知通道，在 Job 失败和 Worker 离线时触发。
7. Web：审计日志查询页面（Table + 筛选）。
8. 测试：审计日志写入/查询、指标计数器、告警触发。

## 验证要求

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
./sdks/java/gradlew -p sdks/java test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
```

完成后更新 `design/tikeo-architecture-design.md`、`.memory/*`、后续 `.prompt/019-*.md`，提交并推送。
