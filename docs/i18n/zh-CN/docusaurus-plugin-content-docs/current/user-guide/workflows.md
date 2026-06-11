# Workflows 运维手册

## 概览

Workflows 页面用于管理 DAG 工作流，包括定义、可视化编辑、JSON/YAML 视图、校验、dry-run、执行、replay、shard 查看和失败节点恢复。运维人员应先校验和 dry-run，再执行会影响生产的工作流。

运维依据：页面由 `web/src/pages/WorkflowsPage.tsx` 提供；主要接口包括 `/api/v1/workflows`、`/api/v1/workflows/{id}`、`/api/v1/workflows/{id}/validate`、`/api/v1/workflows/dry-run`、`/api/v1/workflows/{id}/run`、`/api/v1/workflow-instances/{instance}`、`/api/v1/workflow-instances/{instance}/replay`、`/api/v1/workflow-instances/{instance}/recover`、`/api/v1/workflow-instances/{instance}/shards` 和 `/api/v1/events/instances/{id}/stream`。

## 前置条件

- 具备 `workflows:read` 查看权限；创建、编辑、执行或恢复需要对应管理权限。
- Workflow 中引用的 Job 已存在，并处于正确 namespace/app。
- 相关 Worker 已声明 Job 所需 processor 或 runner。
- 已准备测试输入、回滚方案和失败节点处理策略。

```bash
curl -fsS http://127.0.0.1:9090/api/v1/workflows \
  -H "authorization: Bearer $TIKEO_TOKEN" | jq '.data[] | {id,name,status}'
```

## 打开页面

1. 登录控制台。
2. 在左侧菜单选择 **工作流**，或打开 `/workflows`。
3. 新建工作流使用 `/workflows/new`；编辑现有工作流使用 `/workflows/{id}/edit`。
4. 打开详情后，在画布和定义视图之间切换核对 DAG。

## 常见操作

### 新建小型 DAG

1. 从 start、job、condition、parallel、join、delay、approval、notification、compensation、map、map_reduce 或 sub_workflow 等节点中选择必要节点。
2. 给每个节点设置稳定 key 和清晰名称。
3. Job 节点必须绑定存在且可调度的 Job。
4. 连接 edges，并确认 condition 使用 `always`、`on_success` 或 `on_failure`。
5. 保存前运行 validation。

### 添加通知节点

通知节点不再是 raw webhook target。它会在 Notification Center 中物化 `workflow_node.notification_requested` 消息，并为选中的渠道创建投递尝试。支持两种模式：

- **内联渠道引用**：在 `config.channelRefs` 中选择已注册且启用的 Notification Center channel id；可选 `config.templateRef`、`subject`、`body` 和 `severity`。如果引用的渠道不存在、已禁用，或模板不存在、已禁用、provider 与渠道不匹配，validation/dry-run/create/update 会失败。
- **策略模式**：设置 `config.usePolicies=true`，再创建 `workflow` 或 `workflow_node` 类型的通知策略，由策略匹配 workflow id / node key。

示例：

```json
{
  "key": "notify_ops",
  "kind": "notification",
  "config": {
    "channelRefs": [{"channelId": "notification-channel-ops"}],
    "templateRef": "workflow.node.notice",
    "subject": "Workflow notification requested",
    "body": "A workflow notification node was materialized",
    "severity": "warning"
  }
}
```

旧的 `channel/target/template` raw 字段节点会被校验拒绝，因为它看起来会成功，但不会触达任何已治理渠道。默认投递异步且非阻塞；节点会记录标准化消息和可重试 attempt，工作流继续推进，投递失败进入 Notification Center retry/DLQ。

### 校验和 dry-run

先运行 `/api/v1/workflows/{id}/validate` 或页面校验；新定义可以先用 `/api/v1/workflows/dry-run` 检查 start nodes、node count 和 edge count。校验失败时，不要执行工作流。

### 执行工作流

1. 确认当前定义已保存。
2. 确认引用 Job 有 eligible workers。
3. 点击运行，触发契约为 `triggerType=api`。
4. 记录 workflow instance ID。
5. 到实例视图查看节点状态、shards 和底层 Job instance 日志。

### replay 与节点恢复

replay 用于复盘 workflow instance 的事件和图关系。recover 用于失败节点处理，支持 retry、skip 或 fail。执行前必须确认失败节点、输入 context、下游影响和业务审批。

## 验收

- 可以创建并保存一个包含 Job 节点的小型 DAG。
- validation 能返回通过或明确错误。
- dry-run 能返回 start nodes、node count 和 edge count。
- run 能创建 workflow instance。
- workflow instance 可以查看节点、shards、replay；失败节点可以按权限执行 recovery。
- 每个 Job 节点都能追溯到底层 Instances 日志。

## 故障排查

| 现象 | 处理 |
| --- | --- |
| validation 失败 | 检查孤立节点、非法 edge、重复 key、缺失 Job、不支持的 condition，或 notification 节点缺少 `channelRefs/usePolicies`。 |
| dry-run 结果不符合预期 | 对照定义视图，确认画布位置没有替代真实 DAG 定义。 |
| run 后节点 pending | 到 Jobs 和 Workers 页面检查 Job 可调度性。 |
| shard 失败 | 查看 shard 输入输出，再到 Instances 查底层 Job 日志。 |
| recovery 风险不清楚 | 暂停操作，先导出 replay 证据并确认下游影响。 |

## 生产检查清单

- [ ] 生产工作流执行前已通过 validation 和 dry-run。
- [ ] 每个 Job 节点都有可调度 Worker。
- [ ] recovery 操作有明确的 retry、skip 或 fail 决策依据。
- [ ] notification 节点使用 Notification Center 渠道/模板/策略引用，不包含 raw target 或密钥。
- [ ] replay 证据已保存到工单或事故记录。
- [ ] 不把画布展示当成唯一依据，最终以保存后的 DAG 定义和后端响应为准。
