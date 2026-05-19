# 003-worker-tunnel：Worker 主动连接与注册心跳

> 本阶段提示词需在 002-http-api-and-openapi 完成后根据实际 HTTP/OpenAPI 结构更新。

## 预期目标

- 引入 protobuf/gRPC crate。
- 定义 Worker Tunnel 的最小协议：Register、Heartbeat、ServerMessage。
- 实现 Worker 主动连接的 server 侧连接路由表雏形。
- 不要求 Worker 暴露入站端口。
- 增加最小集成测试或可运行示例。

## 关键设计约束

- Server 不直连 Worker。
- Server→Worker 指令必须复用 Worker 主动建立的双向流。
- Worker identity 以 app、namespace、cluster、region、labels、capabilities 逻辑寻址。
- 所有 Rust 代码位于 `./crates/*`。
- 新依赖默认使用当前最新稳定版。

## 完成后更新

- `.memory/*`
- `.prompt/004-storage-and-scheduler.md`

验证通过后提交并推送。
