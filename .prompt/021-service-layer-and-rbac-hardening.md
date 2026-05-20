# 021-service-layer-and-rbac-hardening

## 背景

020 已修复 015-019 的安全阻断与质量门禁问题，但架构仍偏 handler/repository 直连，RBAC 仍是单字符串 role，ScriptsPage/routes.rs 仍然过大。

## 目标

把 015-020 的功能从“可运行骨架”推进到更适合长期演进的平台结构。

## 范围

1. 拆分 HTTP route 文件：system/auth/users/jobs/scripts/audit。
2. 引入 application service 层：UserService、ScriptService、AuditService。
3. RBAC 设计升级：permission/action/resource 抽象，数据库仍禁止外键，只能软关联。
4. 脚本发布治理：发布指针、回滚 API、审批状态机、Worker 执行版本绑定设计与最小实现。
5. Web：拆分 ScriptsPage 组件，建立 route meta 配置，补统一 401/403 页面。

## 验证要求

沿用 020 全量质量门禁：Rust fmt/clippy/test/build、Java mvn test、Web lint/typecheck/test/build、docker compose config。
