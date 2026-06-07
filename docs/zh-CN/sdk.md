# Tikeo SDK 中文文档 🧩

Tikeo SDK 的目标是“语言不同，功能、规范、运行语义完全一致”。Java、Rust、Go、Python、Node.js 都遵循同一套结构化 Worker 能力、任务日志、脚本沙箱、管理 API 和诊断日志规范。

## 统一能力

- Worker 主动连接服务端 Worker Tunnel。
- 路由只使用结构化能力字段，不使用字符串拼接约定。
- 任务日志必须通过任务上下文输出，避免把非任务日志混入实例日志。
- SDK 诊断日志默认 INFO，输出到控制台，并可配置日志目录写入 `tikeo-sdk.log`。
- 脚本执行必须在明确声明的沙箱中运行。
- 管理接口使用 app 作用域 API-Key。

## SDK 目录

- [Java](../../sdks/java/README.md)
- [Rust](../../sdks/rust/tikeo/README.md)
- [Go](../../sdks/go/tikeo/README.md)
- [Python](../../sdks/python/tikeo/README.md)
- [Node.js](../../sdks/nodejs/tikeo/README.md)
