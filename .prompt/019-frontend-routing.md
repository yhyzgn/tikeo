# 019: 前端路由与导航治理

> 设计文档：`design/frontend-routing-design.md`

## 目标

将当前 `useState('dashboard')` 切换页面的方式替换为 React Router v7 路由系统，实现 URL 持久化、浏览器前进/后退、页面刷新不丢失状态。

## 范围

### 1. 依赖安装

- `bun add react-router react-router-dom`

### 2. 入口改造 (`web/src/main.tsx`)

- 用 `<BrowserRouter>` 包裹 `<App />`

### 3. App.tsx 重构

- 移除 `activePage` 状态、`handleNavigate` 回调
- 移除顶层 jobs/instances 全局加载（各页面独立获取）
- 引入 `<Routes>` 定义路由表：
  - `/login` → LoginPage
  - `/dashboard` → Dashboard
  - `/jobs` → JobsPage
  - `/instances` → InstancesPage
  - `/users` → UsersPage (admin)
  - `/scripts` → ScriptsPage (admin)
  - `/audit` → AuditLogsPage (admin)
  - `/` → 重定向 `/dashboard`
  - `*` → 重定向 `/dashboard`

### 4. AuthGuard 组件 (`web/src/components/AuthGuard.tsx`)

- 检查 authToken，无 token 时重定向 `/login`
- 读取 `/api/v1/auth/me` 恢复 principal
- 提供 principal context 给子组件

### 5. RequireAdmin 组件

- 检查 `roles.includes('admin')`，非 admin 显示 403 或重定向

### 6. AppShell 改造

- 移除 `activeKey` / `onNavigate` props
- 用 `useLocation()` 计算当前高亮菜单项
- 用 `useNavigate()` 处理菜单点击
- 从 AuthGuard context 读取 username/roles

### 7. LoginPage 改造

- 登录成功后 `navigate('/dashboard')`

### 8. 各页面独立数据获取

- Dashboard: 自己 fetch jobs + instances
- JobsPage: 自己 fetch jobs
- InstancesPage: 自己 fetch jobs + instances
- 移除 App.tsx 中通过 props 传递的 jobs/instances/loading/error

### 9. Vite 代理确认

- 确认 `vite.config.ts` 的 `/api` proxy 不与 SPA 路由冲突
- 确认 `historyApiFallback` 配置正确

## 不在范围内

- 详情子页面（`/jobs/:id` 等）
- URL 查询参数持久化筛选
- 路由级别代码分割（React.lazy）
- 面包屑导航

## 验收标准

- [ ] 所有页面通过 URL 直接访问（刷新不丢失）
- [ ] 浏览器前进/后退正常工作
- [ ] 未登录访问任何路由重定向到 `/login`
- [ ] 已登录访问 `/login` 重定向到 `/dashboard`
- [ ] 非 admin 访问 `/users`、`/scripts`、`/audit` 被拦截
- [ ] 菜单高亮与当前 URL 一致
- [ ] cargo clippy / tsc --noEmit / vite build 全部通过
