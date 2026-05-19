# 下一步任务

执行 `.prompt/012-auth-rbac-foundation.md`：

1. 设计最小用户/session/auth 模型，为 Web 登录与权限感知操作打基础。
2. 后端新增 auth 模块与统一 envelope API，避免破坏 `{code,message,data}` 规范。
3. Web 增加登录页/登录态守卫，并在危险操作上体现权限感知。
4. 暂不引入复杂 OIDC；先实现开发态本地账号或 token 机制，并明确后续 OIDC/RBAC 扩展点。
5. 更新设计路线图、`.memory`、后续 `.prompt`，全量验证后提交并推送。
