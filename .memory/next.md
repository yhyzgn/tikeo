# 下一步

## 当前建议阶段

执行 `.prompt/015-user-management-and-rbac.md`。

## 目标

实现完整的账号体系和基础用户管理模块。包括 Users 存储实体、登录认证流、HTTP 接口的 RBAC 鉴权以及 Web UI 上的真实登录与用户管理界面。

## 开始前检查

- 先确认 014-worker-capability-routing 已提交并推送。
- 确认设计文档中提及的安全边界与权限分离约束。
- 接口需要严格遵循 `{code,message,data}` 规范。
- 完善真实登录与路由守卫体系。
- 完成后更新 `.memory/*`、`design/scheduler-architecture-design.md`、新增 `.prompt/016-*.md`。
