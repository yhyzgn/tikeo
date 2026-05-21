# Next Work

当前阶段：024-phase2-distributed-worker-and-recovery 已开发完成，进入验证/提交。

下一阶段建议执行 `.prompt/025-phase2-workflow-worker-results-and-streaming.md`：

1. [x] Worker task_result 与 workflow node 自动回写：job_instance 完成后映射回 workflow_node_instance，并自动 advance 后继节点。
2. Map shard 真正分派给 worker，并支持 shard result、reduce 汇总、失败重试。
3. 子 workflow 完成后自动回写父节点状态。
4. 实时日志流从当前拉取/API 进化到 gRPC/SSE streaming。
5. [partial] 强化队列多节点竞争：已新增 dispatch_queue lease_owner/lease_until、claim/release repository 能力与 HTTP claim API；后续继续强化 DB 原子条件更新 / visibility-timeout。
6. 完整验证并提交推送。

硬约束继续保持：无数据库外键；API envelope 必须 `code/message/data`；禁止 Swagger UI；开发推进后更新 design/.memory/.prompt。
