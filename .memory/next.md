# 下一步

## 当前建议阶段

执行 `.prompt/017-alerting-and-observability.md`。

## 目标

进入告警与可观测性阶段，实现 Prometheus 指标暴露、基础告警通知（邮件/Webhook）、审计日志持久化与查询。

## 开始前检查

- 先确认 016-dynamic-script-sandbox 已提交并推送。
- 脚本管理 CRUD 后端与 Web UI 已完成，但 Worker 侧沙箱执行器尚未实现（留待后续阶段）。
- 接口继续遵循 `{code,message,data}` 规范。
- SessionStore 抽象不被后续模块破坏。
