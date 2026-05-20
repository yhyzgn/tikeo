# 022-phase2-workflow-and-queue-foundation

## 背景

021 先执行 RBAC/service hardening。Phase2「工作流与分布式」顺延到 022。

## 阶段目标

交付 Phase 2 的第一条可运行纵切：DAG 工作流引擎 + 队列/延迟队列基础 + 实时事件流骨架。

## 必做范围

1. DAG 工作流引擎基础：workflow / workflow_node / workflow_edge / workflow_instance / workflow_node_instance 存储模型；禁止外键，只做软关联。
2. DAG 定义校验：节点唯一、边引用存在、禁止环、必须有 start node。
3. Workflow HTTP API：create/list/detail/validate/run/instance detail，响应保持 `{ code, message, data }`。
4. 任务队列与持久化延迟队列基础：dispatch queue model + dispatcher loop。
5. instance event 抽象与 SSE 接口骨架。
6. Web Workflows 菜单和基础 JSON/YAML 定义页面。
7. 更新 design Phase2、.memory、.prompt/023。

## 质量门禁

沿用全量质量门禁：Rust fmt/clippy/test/build、Java mvn test、Web lint/typecheck/test/build、docker compose config；涉及运行时链路时补本地 smoke。
