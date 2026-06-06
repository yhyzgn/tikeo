# Phase4 P2：入站 Webhook / 事件源基础

## 背景
当前 tikeo 已具备告警出站 Webhook 投递能力，但外部系统（CI/CD、监控、Git 平台、内部平台）触发 tikeo Job 仍只能走通用 Job trigger API。Phase4 P2 的“高级 Webhook/事件源”先落地一个稳定、可审计、可用 API-Key 授权的入站事件源基础。

## 本阶段目标
- 新增入站 Webhook 触发入口，外部系统可以 `POST` JSON 事件到 tikeo 并触发指定 Job。
- 鉴权复用现有 session / API Token / SDK API-Key 体系，权限使用 `instances:execute`，并继续校验 namespace/app scope。
- 创建 Job Instance 后写入一条事件源日志，保留 source/eventType/payload 供排障审计。
- 为后续 GitLab/GitHub/Prometheus Alertmanager 等具体事件源适配器保留扩展点。

## API 草案
- `POST /api/v1/events/webhooks/{job}:trigger`
- Body：
  - `source?: string`，默认 `webhook`
  - `eventType?: string`，默认 `webhook.event`
  - `payload?: any`，默认整个请求体去掉 source/eventType 后的对象
- Response：`{ accepted, instanceId, jobId, status, triggerType }`

## 验收
- 后端测试：授权请求触发 Job，实例 triggerType 为 `webhook`，实例日志包含事件源 payload。
- OpenAPI 暴露新 endpoint/schema。
- 前端 API client 提供 `triggerJobWebhookEvent`，测试覆盖路径、body 与 envelope。
- 更新总设计文档 Phase4 P2，标记“Webhook 入站事件源基础”完成，高级 provider 适配器仍保留后续增强。
