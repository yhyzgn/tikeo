# RBAC 角色管理模块实施计划

更新时间：2026-06-07

## 1. 目标与边界

本计划用于把当前“固定角色字符串 + 基础 permission/resource/action”升级为可在 Web 后台配置的生产级 RBAC 角色模块。

### 必须达成

1. **角色可管理**：具备权限的人员可创建、编辑、启停、删除自定义角色；`owner` 为初始化进站账号专属身份和唯一内置兜底角色，必须保留、不可删除、不可编辑、不可禁用、不可手动分配给其他账号；不再内置 `admin` 超级管理员角色；如需普通管理员，可由 owner 在角色管理中自行创建。
2. **初始化账号特权边界**：一次性初始化进站账号（`users.bootstrap_admin = true`）不受普通角色权限约束，作为系统 owner/break-glass 账号；该身份必须在数据层、服务端 principal、前端权限判断中结构化表达，不能靠用户名约定。
3. **用户角色配置**：同时拥有用户管理和角色授权权限的人员，才能调整用户角色；角色变化后必须撤销该用户现有 session，使权限立即刷新。
4. **权限矩阵配置**：角色可配置：
   - 后端接口权限矩阵：基于 `permissions.resource/action` 与接口分组说明。
   - 菜单权限矩阵：基于结构化 menu catalog / menu key，不允许只靠前端路由字符串散落判断。
   - UI 操作元素权限矩阵：在必要页面精确到按钮、表格操作、危险动作、编辑区块等，例如查看、编辑、删除、触发、审批、回滚、复制密钥等；这些能力必须结构化声明并与后端权限联动。
5. **权限感知 UI**：菜单、页面入口、按钮操作继续按 RBAC 控制；新增角色管理页面需要完整 i18n、light/dark 主题、表格/抽屉布局与现有后台风格一致。
6. **兼容已有数据**：现有 `users.role`、`roles`、`permissions`、`role_permissions` 已存在，必须平滑迁移/回填，不破坏当前管理员登录和已有测试数据。

### 非目标

- 不引入数据库外键；继续遵守项目软链接原则。
- 不把 SDK Management API-Key / Service Account 纳入人类用户 RBAC 角色体系；它们继续使用 app-scoped service credential 边界。
- 不使用 JWT 或可解码登录态。

## 2. 当前实现事实

| 领域 | 现状 | 代码位置 |
| --- | --- | --- |
| 角色/权限表 | 已有 `roles`、`permissions`、`role_permissions`，无外键 | `crates/tikeo-storage/src/entities/role.rs`, `role_permission.rs`, `migration/mod.rs` |
| 默认角色 | 迁移里 seed `owner/operator/viewer`；`admin` 不再作为内置默认角色 | `crates/tikeo-storage/src/migration/mod.rs` |
| 用户角色 | `users.role` 单字符串；`CreateUser/UpdateUser/UserSummary` 也是单角色 | `crates/tikeo-storage/src/repository/user.rs` |
| RBAC 查询 | `RbacRepository::permissions_for_role(s)` 只读，无角色 CRUD | `crates/tikeo-storage/src/repository/auth.rs` |
| 用户 API | 角色从角色 catalog 动态校验 enabled role，不再硬编码 owner/admin/operator/viewer | `crates/tikeo-server/src/http/routes/users.rs` |
| 服务端鉴权 | 仅 `bootstrap_admin` 结构化身份绕过；普通角色均走权限矩阵 | `crates/tikeo-server/src/http/services.rs` |
| 前端鉴权 | `principal.bootstrapAdmin` 结构化 bypass；菜单按服务端 menuKeys + route fallback 过滤 | `web/src/components/AuthGuard.tsx`, `web/src/routes.tsx` |
| 用户页面 | 用户创建/编辑角色下拉从角色 API 动态加载 | `web/src/pages/UsersPage.tsx` |
| i18n | 已有独立语言文件，新增文案必须进入 locale 文件 | `web/src/i18n/locales/zh-CN.ts`, `en-US.ts` |

## 3. 总体设计决策

### 3.1 角色身份模型

采用“角色 catalog + 单主角色兼容字段 + user_roles 软关联预留”的本阶段长期兼容方案：

- 保留 `roles` 作为角色 catalog。
- 新增/补齐角色字段：`display_name`、`description`、`builtin`、`enabled`、`created_at`、`updated_at`。
- 新增 `user_roles` 软关联表并回填 `users.role`，作为后续多角色扩展的数据基础；`owner` 绑定仅允许初始化账号保留，不进入普通角色授权流程。
- 本阶段 API/UI 仍保持单个 active role assignment，避免一次性扩大用户管理交互复杂度；服务端权限、菜单、UI action 均基于当前单主角色计算。
- `role-owner` 标记 `builtin=true` 且 `assignable=false`，不可删除、不可编辑、不可降权，避免误伤 owner/break-glass 能力。自定义管理员等价角色由 owner 在角色管理中另建。

> 原因：当前产品用户管理仍是单角色交互，先把角色 catalog、权限矩阵和结构化 owner 兜底做成生产闭环；`user_roles` 保留后续平滑升级到多角色的迁移边界。

### 3.2 初始化账号特权

- 在 `AuthSessionSummary` / `MeResponse` 增加 `bootstrapAdmin: bool`。
- `RbacService::principal_has_permission` 优先判断 `principal.bootstrap_admin == true`，再判断 `principal.permissions`。
- 前端 `hasPermission` 优先判断 `principal.bootstrapAdmin`。
- 不再用 `roles.includes('admin')` 作为绕过条件；owner 角色通过权限矩阵自然获得全部权限。
- 用户管理中对 bootstrap account 增加保护：不能删除；不能删除 bootstrap owner 账号；不能把 bootstrap owner 改成其他角色；即使普通角色权限被误配也仍可进入后台。

### 3.3 权限 catalog 与矩阵

权限矩阵分三层：后端接口权限决定服务端是否放行；菜单权限决定是否显示导航/页面入口；UI 操作元素权限决定页面内部是否显示或启用某个操作元素。三者都必须结构化维护，不能靠零散字符串或仅靠前端隐藏。

#### 后端接口权限

- 继续使用 `permissions(resource, action, description)` 表作为后端接口权限 catalog。
- 新增权限：
  - `roles:read`：查看角色与权限矩阵。
  - `roles:manage`：创建/编辑/删除角色和角色权限。
  - `roles:assign`：给用户绑定/解绑角色。
- 后端路由必须显式映射权限，不允许出现“未登记但可访问”的管理接口。
- 为 Web 矩阵提供 API：返回 permission catalog，包含 `resource`、`action`、`description`、`group`、`affectedEndpoints`。

#### 菜单权限

- 新增结构化 menu catalog（服务端权威）：`menu_key`、`label_key`、`route_path`、`group`、`required_permission`、`default_visible_for_builtin_roles`。
- 新增 `role_menu_permissions` 软关联，或在角色 DTO 中以结构化 `menuKeys` 保存角色可见菜单集合。
- Web 菜单不再只依赖本地 `ROUTE_META.permission` 推导；登录 `/auth/me` 返回 `menuPermissions/menuKeys`，前端用服务端返回值过滤菜单。`ROUTE_META` 只保留渲染元数据和兜底要求。
- 内置 owner 默认拥有全部 menu keys；bootstrapAdmin 前端始终显示所有菜单。

#### UI 操作元素权限

- 新增结构化 UI action catalog（服务端权威）：`element_key`、`menu_key/page_key`、`label_key`、`operation`、`required_permission`、`dangerous`、`description`。
- 示例：`jobs.createButton`、`jobs.editAction`、`jobs.deleteAction`、`jobs.triggerAction`、`scripts.publishButton`、`workflows.rollbackAction`、`apiKeys.revealCreatedKey`。
- 新增 `role_ui_action_permissions` 软关联，或在角色 DTO 中以结构化 `uiActionKeys` 保存可用操作元素集合。
- Web 的 `PermissionGate` / `GuardedButton` 应支持 `uiActionKey`，由 `/auth/me` 返回的 `uiActionKeys` 精确控制元素显示/禁用；同时仍保留 `resource/action` 作为后端兜底。
- UI 操作权限不能替代后端接口权限：即使按钮被隐藏，后端接口仍必须通过 `require_permission` 拦截。
- 矩阵编辑时应提示冲突：勾选 UI 删除按钮但未勾选后端 delete/manage 权限时，应给出 warning 或自动联动建议。

### 3.4 用户授权规则

- 创建/编辑用户时，角色来源必须从角色 API 加载 `assignable=true` 的 active roles，不能自由输入；`owner` 不出现在普通授权下拉中。
- 给用户分配角色需要同时满足：`users:manage` + (`roles:assign` 或 `roles:manage`)。
- 角色变更后调用 `SessionManager::revoke_user_sessions`，强制重新登录刷新权限。
- 删除/禁用角色前必须校验影响：不能删除内置 owner；不能删除仍被用户绑定的角色，除非 API 支持显式 `forceReassignRoleId`。`owner` 始终不可分配给非 bootstrap 用户。

## 4. 实施任务清单

| 阶段 | 任务 | 主要文件 | 验收标准 | 状态 |
| --- | --- | --- | --- | --- |
| A | 数据模型与迁移 | `crates/tikeo-storage/src/entities/*`, `migration/*`, `sqlite_compat.rs` | 新增 role 字段、`user_roles`、菜单权限关联、UI 操作元素权限关联；现有 `users.role` 自动回填；SQLite 兼容测试通过 | 已完成 |
| A | RBAC Repository 拆分 | `crates/tikeo-storage/src/repository/rbac.rs` | 提供角色 CRUD、权限 catalog、角色权限更新、菜单/UI action 查询；避免 auth.rs 继续膨胀 | 已完成 |
| B | DTO/OpenAPI | `crates/tikeo-server/src/http/dto.rs`, `openapi.rs` | 增加 `RoleSummary`、`PermissionCatalogItem`、`MenuPermissionItem`、角色创建/更新请求 | 已完成 |
| B | 角色 API | `crates/tikeo-server/src/http/routes/roles.rs`, `router.rs` | `GET/POST/PATCH/DELETE /api/v1/roles`、权限/menu/UI action catalog、角色权限全量替换；审计覆盖 | 已完成 |
| B | 用户 API 对齐 | `routes/users.rs`, `session.rs`, `auth.rs`, `services.rs` | 用户创建/编辑使用 managed enabled role；bootstrapAdmin 结构化 bypass；角色变更撤销 session | 已完成 |
| C | Web API client | `web/src/api/client.ts`, `client.test.ts` | 角色/权限/menu/UI action catalog API 类型完整；bun 测试覆盖既有 client 契约 | 已完成 |
| C | 角色页面 | `web/src/pages/RolesPage.tsx`, `routes.tsx`, `AppShell.tsx` | 治理菜单新增角色；角色列表、抽屉、后端权限矩阵、菜单矩阵、UI 操作元素矩阵可编辑 | 已完成 |
| C | 用户页面改造 | `web/src/pages/UsersPage.tsx` | 角色从 API 加载 assignable enabled roles；单选当前 active role；bootstrap owner 用户保护展示；owner 不可手动授权；不再硬编码 admin/operator/viewer | 已完成 |
| C | 前端鉴权与菜单/元素 | `AuthGuard.tsx`, `Permission.tsx`, `routes.tsx` | bootstrapAdmin bypass；无 admin 字符串绕过；菜单按服务端 menuKeys + route fallback 控制；按钮/表格操作按 uiActionKeys 精确控制 | 已完成 |
| C | i18n/主题体验 | `web/src/i18n/locales/*`, CSS | 新页面中文/英文文案覆盖；矩阵表格 light/dark 协调 | 已完成 |
| D | 测试与验证 | Rust tests, Bun tests | 存储、HTTP、Web typecheck/API、用户/角色权限、session 刷新、bootstrap bypass 已自动化验证；Playwright 可作为后续视觉 smoke | 已完成 |
| D | 文档与联调数据 | `design/*`, `.prompt/*` | 设计文档与下一阶段提示词已同步 owner 方案；dev DB 是否纳入以实际联调数据为准 | 已完成 |

## 5. API 草案

### 5.1 角色接口

```http
GET    /api/v1/roles
POST   /api/v1/roles
PATCH  /api/v1/roles/{id}
DELETE /api/v1/roles/{id}
GET    /api/v1/permissions/catalog
GET    /api/v1/menu-permissions/catalog
GET    /api/v1/ui-action-permissions/catalog
```

### 5.2 用户接口调整

```json
{
  "username": "ops_alice",
  "email": "ops@example.com",
  "password": "...",
  "roleIds": ["role-operator", "role-tenant-auditor"]
}
```

返回：

```json
{
  "id": "usr-...",
  "username": "ops_alice",
  "roles": [
    { "id": "role-operator", "name": "operator", "displayName": "Operator", "builtin": true }
  ],
  "bootstrapAdmin": false
}
```

### 5.3 `/auth/me` 调整

```json
{
  "username": "bootstrap_admin",
  "roles": ["owner"],
  "bootstrapAdmin": true,
  "permissions": [{ "resource": "users", "action": "manage" }],
  "menuKeys": ["/dashboard", "/users", "/roles"],
  "uiActionKeys": ["users.create", "users.edit", "roles.permissions.edit"]
}
```

## 6. 验证矩阵

| 场景 | 预期 |
| --- | --- |
| 初始化账号尝试改成 viewer | API 拒绝；bootstrap owner 仍可访问角色/用户/API-Key 等全部后台能力 |
| 非 bootstrap 的普通管理员 | 通过自定义角色权限矩阵获得授权；不依赖前端/后端硬编码角色名 bypass |
| 拥有 `users:manage` 但没有 `roles:assign` | 可编辑邮箱/密码，不可调整角色 |
| 拥有 `roles:manage` 但没有 `users:manage` | 可维护普通角色矩阵，不可改用户角色，且不能修改 owner |
| 自定义角色只勾选 jobs read | 只能看到总览/任务等被授权菜单，后端拒绝无权限接口 |
| 修改某用户角色 | 该用户旧 token 立即失效，需要重新登录 |
| 删除被用户绑定的角色 | API 拒绝并返回影响用户数；删除 owner 永远拒绝 |
| 禁用角色 | 已绑定用户重新登录后不再获得该角色权限；当前 session 被撤销或失效 |
| 菜单权限未勾选但接口权限勾选 | 菜单隐藏，但直接访问路由仍由接口权限/Route guard 判定，后端权限不受菜单影响 |
| UI 操作元素权限未勾选但接口权限勾选 | 页面可查看数据，但对应按钮/表格操作隐藏或禁用 |
| UI 操作元素权限勾选但接口权限未勾选 | 矩阵保存时提示冲突；即使前端显示，后端接口仍拒绝 |
| 接口权限未勾选但菜单勾选 | 页面可见但具体操作/数据接口失败；UI 应提示缺少接口权限，建议矩阵保存时给出冲突 warning |

## 7. 风险与缓解

| 风险 | 缓解 |
| --- | --- |
| RBAC 逻辑分散导致绕过 | 建立服务端权限 catalog 与统一 `require_permission`；测试扫描关键路由 |
| 角色编辑误伤 owner | `role-owner` 内置锁定且 assignable=false，不允许删除/禁用/降权/授权给他人；bootstrapAdmin 独立兜底 |
| 前端菜单与后端权限不一致 | 服务端返回 menu catalog/menuKeys；Web 只渲染结构化 key，不自行发明权限 |
| 迁移破坏旧用户 | `users.role` 回填 `user_roles`，测试覆盖已有 owner/operator/viewer 与历史 role 回填 |
| 文件继续膨胀 | 新增 `routes/roles.rs`、`repository/rbac.rs`、Web 组件拆分为列表、基础信息、权限矩阵、菜单矩阵 |

## 8. 推荐执行顺序

1. 数据迁移 + repository 单元测试。
2. 后端角色 API + permission/menu catalog + OpenAPI + HTTP 集成测试。
3. Auth/session principal 增加 bootstrapAdmin/menuKeys + 用户 API 角色绑定改造。
4. Web API client + 角色管理页面 + 用户页面角色选择改造。
5. 权限感知菜单与 route guard 调整。
6. i18n、主题、Playwright 浏览器验收。
7. 更新总设计文档、README、dev 联调数据；提交并推送。
