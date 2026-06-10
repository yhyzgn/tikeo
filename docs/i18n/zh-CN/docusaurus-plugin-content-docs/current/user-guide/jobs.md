# Jobs 用户指南

Jobs 页面由 `web/src/pages/JobsPage.tsx` 实现。它管理 Job 定义、调度类型、namespace/app scope、script/plugin/SDK processor binding、版本历史、rollback、API 触发、广播选择器触发和调度建议。

## 源码对应的数据路径

页面使用 `/api/v1/jobs` 列表与创建 Job，使用 `/api/v1/jobs/{job}` 更新或删除 Job，使用 `/api/v1/jobs/{job}:trigger` 启动 API 触发实例，使用 `/api/v1/jobs/{job}/versions` 与 `/api/v1/jobs/{job}/rollback` 管理版本，使用 `/api/v1/jobs/{job}/scheduling-advice` 检查容量。Worker processor 选项来自 Worker Tunnel snapshot。

## 创建和编辑 Job

先选择 namespace 与 app，因为后续路由、canary target 校验和 service-account 权限都依赖 scope。编辑抽屉允许 scope move，但后端必须同时授权源 scope 与目标 scope。Processor binding 要显式：SDK processor 来自 Worker structured capabilities，script 来自 approved script，plugin 来自 enabled plugin processor definition。

## 触发与广播执行

默认 API trigger 路径使用 `triggerType=api` 与 `executionMode=single`。广播执行必须通过页面的广播抽屉和 `broadcastSelector` payload 显式选择。只有当 Worker 真实声明对应 structured capabilities 或 labels 时，才使用 tags、region、cluster 或 labels；不要依赖 Job 名称约定。

## 验证和排障

保存前检查 schedule type、retry policy、calendar、canary target 与 worker pool。触发前打开 scheduling advice 确认 eligible workers。触发后进入 Instances，用 `/api/v1/instances/{instance}` 和 `/api/v1/instances/{instance}/logs` 确认结果与日志证据。


## 验收检查清单

页面内容必须和源码保持一致：Job 创建、编辑、删除、触发、广播、版本、回滚和调度建议都由接口返回结果决定，前端不保存影子状态。验收时至少检查一个单机 API 触发 Job、一个带 `broadcastSelector` 的广播 Job、一个跨 namespace/app scope 编辑失败案例，以及一个调度建议无 eligible worker 的失败提示。若这些证据缺失，应补齐测试或文档，而不是收缩验收范围。
