# 风险与验证缺口

- UI 技术栈已固定为 React + TypeScript + Ant Design + Bun，但工程尚未初始化。
- 数据库 schema、更多 crate 拆分、protobuf 包名需要在后续阶段继续落地。

- 依赖安全审计命令尚未配置；bootstrap 阶段应考虑 cargo-deny/cargo-audit 与 Bun 依赖审计替代方案。

- 数据库 schema、crate 拆分、protobuf 包名需要在后续阶段继续落地。
- Job persistence 尚未实现；`POST /api/v1/jobs` 当前返回 501 placeholder。
- OpenAPI JSON 路径为 `/api-docs/openapi.json`，不是早期提示词里的 `/openapi.json`。
- Worker Tunnel 当前只有注册/心跳 skeleton，尚未实现真实任务分发、取消、drain、证书轮换。
- Worker Tunnel 当前 smoke 只验证 9091 监听与单元测试，尚未加入真实 gRPC client 集成测试。