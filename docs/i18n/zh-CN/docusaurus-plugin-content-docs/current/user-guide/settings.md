# Settings 与治理指南

Settings 相关控制台界面由 `web/src/routes.tsx` 定义，而不是一个单体 settings 页面。当前 route 包括 users、roles、tenant scopes、API-Key management、calendars、GitOps/IaC、启用时的 OIDC identities，以及 observability/governance entries。

## Route 与 RBAC 来源

`web/src/routes.tsx` 是唯一 route metadata source。菜单项声明 path、label、icon group 和 RBAC resource/action。相同 RBAC 模型会隐藏不可用 action 与 route，因此当 settings 页面不可见时，operator 应先检查权限。

## API-Key 管理

API-Key 与 service-account management 是 app-scoped machine-to-machine governance。SDK Management API 调用使用 `x-tikeo-api-key`，不是 human browser session。凭证不再需要或发生暴露后，应在 API-Key route 中 rotate 或 revoke。

## Tenant scopes 与 roles

Namespace/app scope 控制 Job ownership、service accounts、Worker pools、secrets 和 canary targets。RBAC roles 控制用户对各资源族的 read、write、execute 或 manage 权限。除非用户同时具备源 scope 与目标 scope 授权，否则不要跨 scope 移动 Job。

## 运维边界

Settings route 不是隐藏功能的占位符。如果 route 被 disabled，说明它尚未 production-ready。如果页面存在但 action 被隐藏，应根据 route metadata 与 permission catalog 中要求的 `RBAC` resource/action 决定是否授权，或继续保持不可用。


## 验收检查清单

验收时至少检查普通用户、operator 和 admin 三种权限视角，确认菜单、按钮和 API 返回一致。API-Key 创建后只展示一次明文，后续只展示 prefix；撤销、轮换、service account scope 和 RBAC permission catalog 都必须有后端证据。若 route metadata 与后端权限不一致，应优先修复单一权限源。


## 持续维护要求

后续修改本页面时，必须同时核对对应源码、接口路径、RBAC 行为和自动化测试。文档不能为了看起来完整而描述尚未实现的按钮、字段或后端能力；如果验收发现差异，应把差异转成补丁、测试或明确风险记录。
