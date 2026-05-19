# 风险与验证缺口

- UI 技术栈已固定为 React + TypeScript + Ant Design + Bun，但工程尚未初始化。
- OpenAPI 生成库最终选择 `utoipa` / `aide` / `schemars` 尚未决策。
- 数据库 schema、更多 crate 拆分、protobuf 包名需要在后续阶段继续落地。

- 依赖安全审计命令尚未配置；bootstrap 阶段应考虑 cargo-deny/cargo-audit 与 Bun 依赖审计替代方案。

- 数据库 schema、crate 拆分、protobuf 包名需要在后续阶段继续落地。