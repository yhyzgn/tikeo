# 下一步

## 建议阶段

执行 `021-service-layer-and-rbac-hardening`（待创建）。

## 目标

在 020 善后修复安全阻断后，继续把 015-019 的“能跑骨架”升级为平台级结构：拆分 HTTP routes、引入 application service 层、RBAC permission/resource action 模型、脚本发布指针/回滚/审批状态机、审计分页过滤与告警规则 API。

## 优先事项

1. 拆分 `crates/scheduler-server/src/http/routes.rs`：system/auth/users/jobs/scripts/audit 分文件。
2. 引入 `UserService`、`ScriptService`、`AuditService`，避免 handler 直接编排业务。
3. RBAC 从单字符串 role 演进为 `permission + resource action`，继续保持 DB 软关联。
4. 脚本补发布指针、回滚 API、审批状态机和 Worker 执行版本绑定。
5. Web 拆分 `ScriptsPage` 并引入路由 meta、懒加载和统一 401/403/error 处理。
