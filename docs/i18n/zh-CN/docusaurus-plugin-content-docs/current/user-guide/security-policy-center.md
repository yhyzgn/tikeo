---
title: 安全策略中心
description: Tikeo 中基于真实来源的安全态势、策略证据和部署前置条件操作指南。
---

# 安全策略中心

安全策略中心是已经落地安全控制点的操作员视图。它不是假的策略实验室，也不会执行用户上传的 DSL。页面读取 `/api/v1/security/posture`，并展示来自配置、脚本策略快照、通知渠道脱敏元数据、集群传输配置和审计日志的证据。

## 它回答什么问题

| 问题 | 页面使用的来源 |
| --- | --- |
| 脚本执行策略是否仍然默认拒绝？ | script 行上存储的 `ScriptExecutionPolicy` 快照。 |
| 危险脚本能力是否被阻断？ | Server create/update 校验和 release-gate 审计失败。 |
| 脚本发布签名是否已配置？ | `script_governance.release_signature_secret_ref`。 |
| 通知目标是否已脱敏？ | `notification_channels.target_redacted`、脱敏 config 和 safety policy metadata。 |
| 部署传输边界是否满足可信生产要求？ | HTTP/Worker Tunnel TLS/mTLS 状态和 Raft transport token。 |
| 最近有哪些策略拒绝？ | failed audit event，尤其是 script publish/release-gate denial。 |

## 权限要求

菜单入口和 API 都要求 `security:read`。内置 owner、operator、viewer 角色会通过 Security Policy Center RBAC migration 获得读取权限。`security:manage` 预留给后续托管策略阶段，目前只给 owner seed。

## 态势模型

`GET /api/v1/security/posture` 返回：

- `overallStatus`：由检查项推导出的 `ok`、`warning` 或 `critical`。
- `checks`：每个检查项包含 `id`、`status`、`source`、`detail` 和 `evidenceCount`。
- `scriptGovernance`：统计默认拒绝脚本、危险策略快照、已发布脚本、已签名发布和带 grant evidence 的发布。
- `notificationSafety`：统计已配置/已脱敏目标和 safety policy metadata。
- `clusterTransport`：报告 Raft token 和 TLS readiness。
- `recentDenials`：最近失败的策略/审计事件，包括脚本发布门禁拒绝。

## 状态如何理解

| 状态 | 含义 | 典型动作 |
| --- | --- | --- |
| `ok` | 检查项有来源证据，且本地没有发现问题。 | 保留现有 rollout 证据。 |
| `warning` | 开发环境可能可接受，但生产需要更强的部署前置条件。 | 检查 TLS/mTLS、Raft token、发布签名或网络层配置。 |
| `critical` | 持久化策略证据显示不安全状态。 | 暂停发布，定位受影响脚本/渠道/配置，并用审计日志追溯变更。 |

## 页面展示的部署前置条件

安全策略中心能确认进程内设置，但不能单独证明所有外部网络层属性。生产环境应结合阅读：

- [生产部署](../deployment/production)
- [Server 高可用与 Raft FSOD 集群](../deployment/server-ha)
- [SSE 实时通道部署](../deployment/sse-realtime)
- [配置参考](../reference/configuration)

特别是云 LB/WAF/TLS/多可用区行为仍然需要环境相关 HA 验收；这与本地 Kind 证据是两类检查，当前按任务要求暂不执行真实云验收。

## API smoke check

```bash
curl -fsS \
  -H "Authorization: Bearer $TIKEO_TOKEN" \
  http://127.0.0.1:9090/api/v1/security/posture | jq '.data.overallStatus, .data.checks[] | {id,status,source}'
```

生产候选版本不应有 `critical` 检查。`warning` 只有在目标部署模型中被明确解释时才可接受，例如 TLS 在 Ingress 终止、内部 mTLS 另有 runbook 证明。
