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

- Web build 当前有 Vite 大 chunk 警告（Ant Design bundle），功能构建通过；后续可用动态 import / 路由拆包优化。
- Web 当前是管理端骨架，登录、RBAC、实例日志查看和实时事件流尚未实现。

- Docker/K8s 基础部署已验证；K8s 当前只有原始 YAML 与开发态 SQLite PVC，生产仍需要 Helm Chart、外部数据库、高可用、Ingress/Gateway、NetworkPolicy、PDB、ServiceMonitor。
- Web nginx 代理假设后端服务名为 `scheduler`；Compose 与当前 K8s YAML 已保持该名称，若 Helm/生产命名变化，需要模板化 upstream。

- 009 dispatch loop 当前是单节点 first-available worker 策略，尚未实现 capability/tag 匹配、租约过期剔除、任务 ack 超时、重试、幂等锁和多 server 协调。
- TaskResult 当前只落实例最终状态，尚未持久化 worker_id、错误信息、执行耗时和日志。

- 010 scheduler tick loop 使用内存 cursor，server 重启后可能重新计算到期触发；后续需要持久化 next_fire_at / last_fire_at 与分布式锁。
- CRON / Fixed Rate 当前只创建 pending instance，尚未实现 misfire 策略、时区配置、暂停/恢复、最大并发、任务堆积保护。

- 011 日志当前按实例分页骨架返回全部结果，尚未实现游标分页、日志压缩/归档、实时 SSE/WebSocket 流和敏感信息脱敏策略。
- TaskLog 持久化当前信任 worker 上报的 sequence/level/message，后续需要大小限制、速率限制和租户隔离。

