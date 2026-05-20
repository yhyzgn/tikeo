# 下一步

## 建议阶段

执行 `022-phase2-workflow-and-queue-foundation`。

## 阶段定位

021 已转为 RBAC/service hardening 与后端模块拆分阶段。下一阶段回到用户要求的 Phase2「工作流与分布式」。

## 022 目标

建立 Phase 2 第一条可运行纵切：DAG 工作流引擎 + dispatch queue/持久化延迟队列基础 + instance event/SSE 实时事件流骨架 + Web Workflows 最小入口。

## 优先事项

1. 新增 workflow / workflow_node / workflow_edge / workflow_instance / workflow_node_instance 存储模型；继续禁止外键，只做软关联。
2. 实现 DAG 定义校验：节点唯一、边引用存在、禁止环、start node 存在。
3. 新增 Workflow HTTP API，所有响应保持 `{ code, message, data }` 且 data 必须出现。
4. 引入 dispatch queue / delayed queue 存储模型和 dispatcher loop。
5. 增加 instance event 抽象与 SSE 接口骨架。
6. Web 新增 Workflows 菜单和基础 JSON/YAML 定义页面。
7. 更新 design 路线图、.memory 与 .prompt/023；验证全量质量门禁后提交推送。
