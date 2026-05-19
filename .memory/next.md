# 下一步任务

执行 `.prompt/005-basic-scheduler.md`：

1. 定义基础调度领域模型：ScheduleType、TriggerType、InstanceStatus、DispatchDecision。
2. 扩展 `scheduler-storage`，实现 job_instance repository。
3. 增加 `POST /api/v1/jobs/{job}:trigger`，创建 API 手动触发实例。
4. 增加实例查询接口并更新 OpenAPI。
5. 保持 `{code,message,data}` 响应规范。
6. 保持 HTTP/OpenAPI、storage migration/repository 与 Worker Tunnel 验证通过。
7. 更新设计路线图、`.memory` 和后续 `.prompt`。
8. 提交并推送。
