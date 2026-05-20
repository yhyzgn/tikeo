# 下一步

## 建议阶段

执行 `023-phase2-workflow-visual-and-mapreduce`。

## 阶段定位

022 已完成 Phase2 第一条纵切：DAG 定义存储、校验、最小运行、dispatch_queue/delayed queue 基础、instance_events/SSE 骨架、Web Workflows JSON 入口。

## 优先事项

1. 完善 workflow 执行推进器：根据 node result 和 edge condition 推进后继节点。
2. 实现 Map / MapReduce 执行模式的存储与最小 API。
3. 支持子工作流节点类型。
4. Web 工作流页面从 JSON 表单升级为基础 DAG 可视化编辑器，并补 YAML/JSON 双模式和 dry-run。
5. SSE 接入 Web 实时实例事件展示。
6. 继续保持 API `{ code, message, data }` 和全库无外键。
