# 风险与验证缺口

- 当前尚未初始化代码工程，无法执行 Rust 编译、测试、运行验证。
- 远程 git 仓库与推送权限尚未验证。
- UI 技术栈已固定为 React + TypeScript + Ant Design + Bun，但工程尚未初始化。
- OpenAPI 生成库最终选择 `utoipa` / `aide` / `schemars` 尚未决策。
- 数据库 schema、crate 拆分、protobuf 包名需要在 bootstrap 阶段落地。

- 依赖安全审计命令尚未配置；bootstrap 阶段应考虑 cargo-deny/cargo-audit 与 Bun 依赖审计替代方案。
