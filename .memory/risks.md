# 风险与验证缺口

- UI 技术栈已固定为 React + TypeScript + Ant Design + Bun，但工程尚未初始化。
- 依赖安全审计命令尚未配置；bootstrap 阶段应考虑 cargo-deny/cargo-audit 与 Bun 依赖审计替代方案。

- 基础调度 tick loop、实例状态机和 Worker 任务分发尚未实现；当前只完成 Jobs 持久化与 Worker 注册/心跳 skeleton。
- MySQL migration 已通过 SeaORM feature 启用，但当前自动化验证只覆盖 SQLite in-memory 与 SQLite dev DB，尚未接入真实 MySQL 集成测试。
- OpenAPI JSON 路径为 `/api-docs/openapi.json`，不是早期提示词里的 `/openapi.json`。
- Worker Tunnel 当前只有注册/心跳 skeleton，尚未实现真实任务分发、取消、drain、证书轮换。
- Worker Tunnel 当前 smoke 只验证 9091 监听与单元测试，尚未加入真实 gRPC client 集成测试。
- Axum 0.8 不允许同一路径段内同时使用参数和字面量后缀；`/api/v1/jobs/{job}:trigger` 对外契约由内部 `/jobs/{job_action}` 路由承接并在 handler 中解析 `:trigger` 后缀。
- CRON / Fixed Rate tick loop 尚未实现；当前基础调度只覆盖 API 手动触发实例入库。

- Java SDK 当前只有 Spring Boot Starter 骨架、注解扫描和 Noop client，尚未生成/接入 Java gRPC Worker Tunnel 真实连接。
- Worker Tunnel proto RPC 已从 `Connect` 改为 `OpenTunnel`；外部 SDK 若已基于旧名生成代码需要同步更新。
