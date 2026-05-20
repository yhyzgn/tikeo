# 下一步

## 建议阶段

执行 `021-phase2-workflow-and-queue-foundation`。

## 阶段定位

020 已完成 015-019 善后。用户确认若无其他善后问题，则 021 必须优先进入 `design/scheduler-architecture-design.md` 的 Phase 2「工作流与分布式」，而不是先做 Phase 3 倾向的 RBAC/service hardening。

## 021 目标

建立 Phase 2 第一条可运行纵切：DAG 工作流引擎 + dispatch queue/持久化延迟队列基础 + instance event/SSE 实时事件流骨架 + Web Workflows 最小入口。

## 优先事项

1. 新增 workflow / workflow_node / workflow_edge / workflow_instance / workflow_node_instance 存储模型；继续禁止外键，只做软关联。
2. 实现 DAG 定义校验：节点唯一、边引用存在、禁止环、start node 存在。
3. 新增 Workflow HTTP API，所有响应保持 `{ code, message, data }` 且 data 必须出现。
4. 引入 dispatch queue / delayed queue 存储模型和 dispatcher loop，调度 tick 不再直接强耦合 worker dispatch。
5. 增加 instance event 抽象与 SSE 接口骨架，为后续 WebSocket/gRPC stream 做准备。
6. Web 新增 Workflows 菜单和基础 JSON/YAML 定义页面。
7. 更新 design 路线图、.memory 与 .prompt/022；验证全量质量门禁后提交推送。

## 暂缓事项

- `021-service-layer-and-rbac-hardening` 已改为后移计划，建议 Phase 2 基础稳定后作为 024 或 Phase 3 前置治理执行。
- 真正 Raft、多数据库 PostgreSQL/CockroachDB、完整拖拽 DAG 编辑器、Go/Python SDK 可拆到 022+。
