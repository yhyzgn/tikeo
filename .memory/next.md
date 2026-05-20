# Next Work

当前阶段：023-phase2-workflow-visual-and-mapreduce 已进入验证/收尾。

下一阶段建议执行 `.prompt/024-phase2-distributed-worker-and-recovery.md`：

1. 将 workflow queued node 与 worker dispatch 真正打通：job 节点创建 job_instance，map/map_reduce 生成 shard/subtask，sub_workflow 节点触发 child workflow instance。
2. 增强失败恢复：节点重试、跳过、从失败节点恢复、实例回放。
3. Worker 集群页面与 API：在线 worker、能力标签、心跳、隧道状态、队列积压。
4. 完整验证：cargo fmt/clippy/test/build、mvn test、bun lint/typecheck/test/build、docker compose config。

硬约束继续保持：无数据库外键；API envelope 必须 `code/message/data`；禁止 Swagger UI；开发推进后更新 design/.memory/.prompt 并提交推送。
