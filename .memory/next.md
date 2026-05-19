# 下一步

## 当前建议阶段

执行 `.prompt/014-worker-capability-routing.md`。

## 目标

在 013 已完成的 single/broadcast 执行闭环基础上，实现 Worker 能力 / 标签 / namespace / app 的基础路由，让任务只发送给符合条件的在线 Worker。

## 开始前检查

- 先确认 013-broadcast-execution 已提交并推送。
- 禁止使用浏览器 API 文档 UI；仅保留 `/api-docs/openapi.json` 作为机器可读接口契约。
- 保持 HTTP API envelope：`{code,message,data}`，`data` 为 null 时也必须返回。
- Worker 仍只能主动建立 `OpenTunnel`；Server 不得直接回连 Worker。
- Docker/Compose 验证必须使用默认 bridge 网络，不得用 host 网络规避网络层问题。
- 完成后更新 `.memory/*`、`design/scheduler-architecture-design.md`、新增 `.prompt/015-*.md`。
