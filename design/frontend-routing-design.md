# 前端路由与导航治理

## 1. 现状问题

当前 Web 端（`web/src/App.tsx`）使用 `useState('dashboard')` 管理 activePage，通过 `onNavigate(key)` 回调切换页面组件。所有页面状态仅存在于内存中，存在以下问题：

- 刷新页面回到 Dashboard，用户深度导航状态丢失
- 无法通过 URL 分享特定页面（如审计日志、脚本详情）
- 浏览器前进/后退按钮无效
- 无法打开多个标签页分别查看不同页面
- 未来子页面（如 `/jobs/:id`、`/scripts/:id`）无法支持

## 2. 技术选型

| 方案 | 优点 | 缺点 |
|------|------|------|
| **React Router v7** (推荐) | 社区标准、嵌套路由、loader/action、TypeScript 支持好 | 包体积略大 |
| TanStack Router | 类型安全路由 | API 变动快、社区较小 |
| 状态驱动 hash 路由 | 零依赖 | 功能弱、不支持 SSR |

选择 **React Router v7**，与 React + TypeScript 技术栈一致，社区成熟。

## 3. 路由表设计

### 3.1 路由结构

```
/                       → 重定向到 /dashboard
/dashboard              → 总览
/jobs                   → 任务列表
/jobs/:id               → 任务详情（未来）
/instances              → 实例列表
/instances/:id          → 实例详情（未来）
/users                  → 用户管理（Admin）
/scripts                → 脚本管理（Admin）
/scripts/:id            → 脚本详情（未来）
/audit                  → 审计日志（Admin）
/login                  → 登录页（无需认证）
```

### 3.2 路由守卫

```
未登录 → 任何路由 → 重定向 /login
已登录 → /login → 重定向 /dashboard
已登录 → Admin 路由（/users, /scripts, /audit）→ 检查 roles.includes('admin')，否则 403 页面
```

### 3.3 菜单与路由映射

| 菜单 key | 路由 path | 权限 | 组件 |
|----------|-----------|------|------|
| dashboard | /dashboard | 登录即可 | Dashboard |
| jobs | /jobs | 登录即可 | JobsPage |
| instances | /instances | 登录即可 | InstancesPage |
| users | /users | admin | UsersPage |
| scripts | /scripts | admin | ScriptsPage |
| audit | /audit | admin | AuditLogsPage |

## 4. 实现方案

### 4.1 依赖变更

```bash
bun add react-router react-router-dom
```

### 4.2 文件变更清单

| 文件 | 变更 |
|------|------|
| `web/src/App.tsx` | 引入 BrowserRouter / Routes / Route，替换 activePage 状态 |
| `web/src/components/AppShell.tsx` | 使用 useNavigate / useLocation 替代 onNavigate/activeKey props |
| `web/src/components/AuthGuard.tsx` | 新建：认证守卫 + Admin 守卫组件 |
| `web/src/main.tsx` | 包裹 BrowserRouter |
| `web/src/pages/LoginPage.tsx` | 登录成功后 navigate('/dashboard') 替代回调 |

### 4.3 App.tsx 重构要点

```tsx
<Routes>
  <Route path="/login" element={<LoginPage />} />
  <Route element={<AuthGuard />}>
    <Route element={<AppShell />}>
      <Route path="/dashboard" element={<Dashboard />} />
      <Route path="/jobs" element={<JobsPage />} />
      <Route path="/instances" element={<InstancesPage />} />
      <Route path="/users" element={<RequireAdmin><UsersPage /></RequireAdmin>} />
      <Route path="/scripts" element={<RequireAdmin><ScriptsPage /></RequireAdmin>} />
      <Route path="/audit" element={<RequireAdmin><AuditLogsPage /></RequireAdmin>} />
    </Route>
  </Route>
  <Route path="/" element={<Navigate to="/dashboard" replace />} />
  <Route path="*" element={<Navigate to="/dashboard" replace />} />
</Routes>
```

### 4.4 AppShell 重构要点

- 移除 `activeKey` / `onNavigate` props
- 使用 `useLocation()` 获取当前路径高亮菜单项
- 使用 `useNavigate()` 处理菜单点击导航
- 菜单项定义与路由 path 对齐

### 4.5 数据获取策略

当前 App.tsx 在顶层加载 jobs + instances 并通过 props 传递。路由化后：

- Dashboard 和 JobsPage 各自独立获取数据（useEffect 内调用 API）
- 移除 App.tsx 中的全局 jobs/instances 状态
- 各页面组件自管理自己的 loading/error 状态

### 4.6 Vite 开发服务器代理

确认 `vite.config.ts` 的 proxy 配置覆盖所有 `/api` 前缀，避免 SPA 路由与 API 路由冲突。

## 5. 不在本次范围内

- 详情子页面（`/jobs/:id`、`/scripts/:id`）— 后续 Phase
- URL 查询参数持久化筛选条件
- 路由级别代码分割（React.lazy）
- 面包屑导航
