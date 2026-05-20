# 021-service-layer-and-rbac-hardening

> 状态：已让位。用户明确要求 021 先做 Phase 2 工作项，因此本文件不再作为 021 主计划执行。

## 后移原因

020 后曾计划在 021 进行 routes/service/RBAC hardening，但 `design/scheduler-architecture-design.md` 中该类 RBAC/企业治理更接近 Phase 3。当前应优先推进 Phase 2「工作流与分布式」。

## 新的 021 主计划

请执行：`.prompt/021-phase2-workflow-and-queue-foundation.md`

## 本计划后续归属

可改为 024 或 Phase 3 前置治理阶段，建议在 Phase 2 的 workflow/queue/event 基础稳定后再做：

1. 拆分 HTTP route 文件：system/auth/users/jobs/scripts/audit/workflows/events。
2. 引入 application service 层：UserService、ScriptService、AuditService、WorkflowService、QueueService。
3. RBAC 设计升级：permission/action/resource 抽象，数据库仍禁止外键，只能软关联。
4. Web：建立 route meta 配置，补统一 401/403 页面。
