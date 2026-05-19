# 下一步任务

执行 `.prompt/011-instance-logs.md`：

1. 增加实例执行日志的存储模型与 migration。
2. 扩展 Worker Tunnel / SDK，让 Worker 可回传任务日志片段。
3. 增加 HTTP API 查询实例日志，仍遵守 `{code,message,data}` envelope。
4. Web Instances 页面增加实例日志查看入口。
5. 保持 Worker outbound-only tunnel 模型，日志不得要求 Worker 暴露入站端口。
6. 增加 Rust storage/server/SDK 测试与 Web API client/UI 基础测试。
7. 完成 cargo、maven、bun、docker/compose 验证后提交并推送。
