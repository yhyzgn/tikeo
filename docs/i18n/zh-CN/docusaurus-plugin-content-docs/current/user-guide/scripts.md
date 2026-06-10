# Scripts 用户指南

Scripts 页面由 `web/src/pages/ScriptsPage.tsx` 实现。它管理 script draft、execution policy、approval/publish flow、rollback、version history，以及源码和策略 `diff` preview。

## 源码对应的数据路径

页面使用 `/api/v1/scripts` 列表与创建，使用 `/api/v1/scripts/{id}` 读取、更新和删除，使用 `/api/v1/scripts/{id}/publish`、`/api/v1/scripts/{id}/rollback`、`/api/v1/scripts/{id}/versions` 与 `/api/v1/scripts/{id}/diff`。Job 只能绑定发布后的 approved script。

## 执行策略

表单暴露 timeout、memory、output limit、environment variables、filesystem、network、secrets 和 sandbox backend。安全默认值是默认拒绝网络/文件系统，并设置资源上限。Worker 必须广告匹配且可执行的 script runner，script 才应被调度过去。

## Draft、diff、publish、rollback

保存或发布前使用 preview 检查内容和策略 diff。发布会创建派发使用的 immutable version。Rollback 会切换到之前的 approved version，也应视为生产变更，因为正在运行的 Job 可能绑定新的 release pointer。

## 边界

Server 只派发 script metadata 和 immutable content；不执行用户代码。执行发生在 Worker 控制的 sandbox 中。如果没有合格 Worker，应修复 Worker runtime capabilities，而不是放宽策略或假装 script 可以运行。


## 验收检查清单

验收时至少覆盖草稿保存、策略变更 diff、发布、版本列表、rollback 和删除权限。对于 sandbox、network、filesystem、secrets 等字段，要确认默认拒绝与资源上限被保存到后端，并且只有具备真实 runner 的 Worker 才广告对应能力。发现缺口时应补实现或记录真实阻塞，不能把不可执行能力写成已支持。


## 持续维护要求

后续修改本页面时，必须同时核对对应源码、接口路径、RBAC 行为和自动化测试。文档不能为了看起来完整而描述尚未实现的按钮、字段或后端能力；如果验收发现差异，应把差异转成补丁、测试或明确风险记录。
