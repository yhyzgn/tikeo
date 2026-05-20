# 025-phase2-workflow-worker-results-and-streaming

## 背景

024 已让 workflow queued node 可物化为 job_instance、workflow_shards 或 child workflow instance，并新增恢复 API、Worker/dispatch queue 管理页面。

## 目标

把 worker 执行结果与 workflow 状态机进一步打通，减少手动推进，使 DAG 能根据 job/shard/sub-workflow 结果自动流转。

## 工作项

1. Worker `TaskResult` 落库 job_instance 后，查找绑定的 workflow_node_instance 并自动更新节点状态。
2. 节点成功/失败后自动调用 workflow advance，按边条件推进后继节点。
3. Map shard 最小分派：为每个 shard 创建可追踪任务或队列项，支持 shard result 写入 output/status。
4. MapReduce reduce 节点在所有 map shards 成功后自动入队。
5. 子 workflow instance 终态映射回父 workflow node。
6. 实时日志流基础：将 job logs / workflow events 暴露为 SSE/gRPC streaming。
7. 队列 claim/lease/visibility-timeout 设计并实现最小字段，降低多 server 重复消费风险。

## 验证

必须运行：cargo fmt/clippy/test/build、mvn test、bun lint/typecheck/test/build、docker compose config。
