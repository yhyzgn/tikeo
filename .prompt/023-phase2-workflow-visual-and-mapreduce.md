# 023-phase2-workflow-visual-and-mapreduce

## 背景

022 已交付 DAG workflow 基础：定义存储、校验、最小 run API、dispatch_queue/run_after、instance_events/SSE 和 Web JSON 入口。

## 目标

继续推进 Phase2：执行推进器、Map/MapReduce、子工作流、Web DAG 可视化和实时事件展示。

## 范围

1. workflow executor：根据 node instance 状态和 edge condition 推进后继节点。
2. Map / MapReduce 执行模式：定义模型、API、最小执行语义。
3. 子工作流节点类型：workflow node 可引用 child workflow。
4. Web DAG 可视化编辑器基础，支持 JSON/YAML 双模式与 dry-run 校验。
5. Web 接入 SSE instance events。
6. 更新 design/.memory，并通过全量质量门禁后提交推送。
