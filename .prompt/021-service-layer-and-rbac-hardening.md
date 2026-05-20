# 021-service-layer-and-rbac-hardening

## 背景

020 已修复 015-019 的安全阻断与质量门禁问题。用户确认 021 先完成 RBAC/service hardening，Phase2 工作流与分布式阶段顺延到下一阶段。

当前架构仍偏 handler/repository 直连，RBAC 仍是单字符串 role，Web 权限感知和 401/403 体验仍有提升空间。

## 目标

把 015-020 的用户管理/认证能力从“基础可运行”升级为可长期演进的平台权限结构。

## 范围

1. RBAC 设计升级：permission/action/resource 抽象，数据库仍禁止外键，只能软关联。
2. 增加角色、权限、角色权限绑定的最小存储模型与 repository/service API。
3. HTTP 鉴权从单纯 role 校验升级为 permission check，继续兼容初始化 admin。
4. 拆出最小 application service 层：至少 UserService/AuthService/RbacService，避免 handler 继续堆业务编排。
5. Web：建立 route meta 权限配置，补统一 401/403 页面或提示，权限不足时隐藏/禁用危险操作。
6. 更新 `design/auth-session-design.md`、`design/scheduler-architecture-design.md` 和 `.memory/*`。

## 非目标

- 不做 Phase2 DAG/queue/工作流实现；已顺延到 `.prompt/022-phase2-workflow-and-queue-foundation.md`。
- 不引入数据库外键。
- 不提供 Swagger UI。

## 验证要求

沿用 020 全量质量门禁：Rust fmt/clippy/test/build、Java mvn test、Web lint/typecheck/test/build、docker compose config。涉及鉴权链路需补 auth/users/rbac targeted tests。
