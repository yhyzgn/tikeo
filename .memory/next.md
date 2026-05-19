# 下一步任务

执行 `.prompt/010-scheduler-tick-loop.md`：

1. 为 `cron` 与 `fixed_rate` Job 实现最小 tick loop，自动创建 pending job_instance。
2. 复用 009 已完成的 dispatch loop：自动触发后的实例应进入 Worker Tunnel 分发与执行回传链路。
3. 补充 schedule expression 校验与下一次触发时间计算的基础模型。
4. 保持 API 响应 `{code,message,data}` 契约和 root binary / crates workspace 结构。
5. 增加 storage/server 单元测试，覆盖 cron/fixed_rate 自动触发与不会重复触发。
6. 更新设计路线图、`.memory`、后续 `.prompt`，完成全量验证后提交并推送。
