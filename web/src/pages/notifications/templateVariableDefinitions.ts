export interface TemplateVariableDefinition {
  placeholder: string;
  label: string;
  description: string;
  example: string;
  source: string;
}

export const STANDARD_TEMPLATE_VARIABLES: TemplateVariableDefinition[] = [
  { placeholder: '{{subject}}', label: '通知主题', description: '消息标题或摘要，适合放在机器人卡片标题、邮件主题和 PagerDuty summary。', example: 'Tikeo job billing-sync: failed', source: '标准消息字段' },
  { placeholder: '{{body}}', label: '通知正文', description: '面向接收人的正文说明，通常包含失败原因或执行摘要。', example: 'Job billing-sync instance inst_01 emitted job_instance.failed: exit 2', source: '标准消息字段' },
  { placeholder: '{{eventType}}', label: '事件类型', description: '规范化事件名，可用于路由、标题前缀或消息分组。', example: 'job_instance.failed', source: '标准消息字段' },
  { placeholder: '{{resourceType}}', label: '资源类型', description: '触发消息的资源类别。', example: 'job', source: '标准消息字段' },
  { placeholder: '{{resourceId}}', label: '资源 ID', description: '触发消息的资源标识。', example: 'job_01HX...', source: '标准消息字段' },
  { placeholder: '{{severity}}', label: '严重级别', description: '策略或事件计算出的通知级别。', example: 'critical', source: '标准消息字段' },
  { placeholder: '{{messageId}}', label: '消息 ID', description: 'Notification Center 消息流水 ID；预览时使用占位值。', example: 'ntmsg_01HX...', source: '标准消息字段' },
  { placeholder: '{{policyId}}', label: '策略 ID', description: '命中的通知策略 ID。', example: 'ntpol_01HX...', source: '标准消息字段' },
  { placeholder: '{{dedupeKey}}', label: '去重键', description: '用于去重窗口和外部平台去重的稳定键。', example: 'ntpol_01:inst_01:job_instance.failed', source: '标准消息字段' },
  { placeholder: '{{triggeredAt}}', label: '触发时间', description: '事件触发或消息创建时间，RFC3339 格式。', example: '2026-06-13T00:00:00Z', source: '标准消息字段' },
  { placeholder: '{{createdAt}}', label: '创建时间', description: '消息创建时间；当前与 triggeredAt 等价。', example: '2026-06-13T00:00:00Z', source: '标准消息字段' },
];

export const PAYLOAD_TEMPLATE_VARIABLES: TemplateVariableDefinition[] = [
  { placeholder: '{{namespace}}', label: 'Namespace', description: '任务、应用或策略所属命名空间。', example: 'prod', source: '事件 payload 顶层字段' },
  { placeholder: '{{app}}', label: '应用', description: '任务所属应用。', example: 'billing', source: '事件 payload 顶层字段' },
  { placeholder: '{{jobId}}', label: '任务 ID', description: '触发事件的 Job ID。', example: 'job_01HX...', source: '事件 payload 顶层字段' },
  { placeholder: '{{jobName}}', label: '任务名称', description: '触发事件的 Job 展示名称。', example: 'billing-sync', source: '事件 payload 顶层字段' },
  { placeholder: '{{instanceId}}', label: '实例 ID', description: '本次执行实例 ID，用于追踪实例详情和日志。', example: 'inst_01HX...', source: '事件 payload 顶层字段' },
  { placeholder: '{{status}}', label: '执行状态', description: '实例当前状态或预览事件状态。', example: 'failed', source: '事件 payload 顶层字段' },
  { placeholder: '{{triggerType}}', label: '触发类型', description: '实例触发来源。', example: 'api', source: '事件 payload 顶层字段' },
  { placeholder: '{{executionMode}}', label: '执行模式', description: '实例执行方式。', example: 'single', source: '事件 payload 顶层字段' },
  { placeholder: '{{startedAt}}', label: '开始时间', description: '实例创建/开始时间，RFC3339 格式。', example: '2026-06-13T00:00:00Z', source: '事件 payload 顶层字段' },
  { placeholder: '{{finishedAt}}', label: '结束时间', description: '实例最近更新时间或完成时间，RFC3339 格式。', example: '2026-06-13T00:03:27Z', source: '事件 payload 顶层字段' },
  { placeholder: '{{workerId}}', label: 'Worker ID', description: '上报执行结果的 Worker ID；尚未分配时为空。', example: 'worker-prod-a-01', source: '事件 payload 顶层字段' },
  { placeholder: '{{operatorName}}', label: '操作人', description: '触发或物化通知的操作者名称；系统事件通常为 tikeo。', example: 'tikeo', source: '事件 payload 顶层字段' },
  { placeholder: '{{operatorType}}', label: '操作人类型', description: '操作者类型，例如 system、user 或 api_key。', example: 'system', source: '事件 payload 顶层字段' },
  { placeholder: '{{reason}}', label: '失败/状态原因', description: '失败原因、重试说明或状态摘要；成功/普通状态可能为空或为短横线。', example: '参数不能为空 should not be empty', source: '事件 payload 顶层字段' },
  { placeholder: '{{logsUrl}}', label: '执行日志链接', description: '兼容变量，当前指向免登录实例执行控制台。', example: '/public/instances/inst_01HX/console', source: '事件 payload 顶层字段' },
  { placeholder: '{{consoleUrl}}', label: '公开控制台链接', description: '通知卡片“查看控制台”按钮使用的免登录执行透传页面链接。', example: '/public/instances/inst_01HX/console', source: '事件 payload 顶层字段' },
  { placeholder: '{{templateRef}}', label: '模板引用 ID', description: '策略引用的存储模板 ID；仅引用模板渲染后存在。', example: 'nttpl_01HX...', source: '事件 payload 顶层字段' },
  { placeholder: '{{templateKey}}', label: '模板 Key', description: '策略引用的存储模板业务 Key；仅引用模板渲染后存在。', example: 'ops.job.failure', source: '事件 payload 顶层字段' },
];
