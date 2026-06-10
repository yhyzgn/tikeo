# Workflows 用户指南

Workflows 页面由 `web/src/pages/WorkflowsPage.tsx` 实现。它管理 DAG 定义、可视化预览与编辑、JSON/YAML 定义视图、validation、dry-run、执行、replay、shard 查看和 workflow node recovery。

## 源码对应的数据路径

页面使用 `/api/v1/workflows` 进行列表与创建，使用 `/api/v1/workflows/{id}` 读取与更新，使用 `/api/v1/workflows/{id}/validate` 校验，使用 `/api/v1/workflows/dry-run` dry-run，使用 `/api/v1/workflows/{id}/run` 执行。运行时视图还使用 workflow-instance endpoints 与 Worker event streams。

## DAG 模型

Workflow 是由 nodes 与 edges 构成的 `DAG`。支持的 node kind 包括 job、condition、parallel、join、delay、approval、notification、compensation、map、map_reduce 与 sub_workflow。UI 把视觉坐标存放在 node config 中，同时保留可执行 definition。

## 安全编辑流程

加载现有 Workflow 后，保存前先运行 validation，并比较 definition diff。新建 Workflow 时，从小型 job-backed DAG 开始，先 dry-run 再执行。当 node 引用 Job 时，该 Job 必须在预期 namespace/app，并且有合格 Worker。

## 运行时排障

执行后查看 workflow instance 状态、shards、replay 或 recovery 结果。Recovery 是运维工具；使用前要确认失败 node、输入 context 和下游影响。底层 Job 执行日志请到 Instances 页面查看。


## 验收检查清单

验收时至少覆盖创建小型 DAG、validation、dry-run、保存、执行、查看 workflow instance、查看 shards，以及失败节点 recovery 或 replay 的证据。每个 Job 节点都要能追溯到 Instances 日志。若 DAG 只在前端画布存在而后端无法 validate 或 run，应视为功能未完成。


## 持续维护要求

后续修改本页面时，必须同时核对对应源码、接口路径、RBAC 行为和自动化测试。文档不能为了看起来完整而描述尚未实现的按钮、字段或后端能力；如果验收发现差异，应把差异转成补丁、测试或明确风险记录。
