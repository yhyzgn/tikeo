# 015-user-management-and-rbac

## 背景

原计划进行动态脚本执行沙箱的开发，但目前系统缺乏正式的账号体系（用户管理、RBAC 权限验证），而登录和权限功能是支撑多租户和后续安全策略的必需要件。

## 目标

实现完整的账号体系和基础用户管理模块。包括：
- 数据库表：Users、Roles 等关联实体。
- HTTP 接口：用户的增删改查、分配角色。
- 认证流：基于密码或 Token 的真实登录机制（替换或加强目前的临时 token）。
- RBAC 验证：根据用户的 role 拦截或允许 API 请求。
- Web UI：实现真实登录界面，添加用户管理界面（允许 Admin 查看并管理用户和角色）。

## 开始前检查

- 确认 `crates/tikeo-storage` 引入 Users 相关 Entity 并生成 migration。
- 确认 `crates/tikeo-server/src/http/auth.rs` 能够解析真实 Token 并检查 DB 或内存缓存以提供完整的 Authentication 和 Authorization。
- 完善 `web/src/pages/LoginPage.tsx` 以及 `api/client.ts` 使其对接真实登录接口。
- 完成后更新 `.memory/*`、`design/tikeo-architecture-design.md`、新增 `.prompt/016-*.md`。

## 验证要求

- 可以通过 `admin` 用户登录系统。
- 能够创建一个 `viewer` 角色用户，并验证该用户无法执行 trigger 或 create 操作（收到 403 Forbidden）。
- 确保整个工作流的通过，包括 `cargo test` 和 frontend lint/typecheck。
