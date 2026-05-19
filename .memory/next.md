# 下一步任务

从 `.prompt/001-bootstrap.md` 开始进入代码开发阶段：

1. 初始化 Rust workspace，所有 Rust crate 必须位于 `./crates/`。
2. 建立基础 crate 拆分。
3. 增加 CLI serve 子命令。
4. 增加 Axum healthz/readyz 最小服务。
5. 建立基础配置加载。
6. 建立 CI 与本地验证命令。
7. 运行 fmt、clippy、test、build、healthz 冒烟。
8. 更新 `.memory` 和 `.prompt/002-http-api-and-openapi.md`，并保持后续 `006-web-ui-foundation` 的 `./web` + React + Ant Design + Bun 约束。
9. 提交并推送。
