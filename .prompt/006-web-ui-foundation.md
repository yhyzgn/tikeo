# 006-web-ui-foundation：Web 管理端基础工程

> 本阶段提示词需在后端基础 API 和 OpenAPI 初步稳定后由执行智能体根据实际代码结构更新。

## 硬性约束

- Web 代码目录固定为 `./web/`。
- 技术栈固定为 React + TypeScript + Vite + Ant Design。
- 包管理器固定使用 Bun。
- React、Vite、Ant Design、测试与 lint 相关依赖默认使用当前最新稳定版；不能使用最新版时必须记录原因。
- API client 应从 OpenAPI 生成或基于 OpenAPI 类型约束封装。
- 浏览器不得直接访问 Worker，只能访问 scheduler server HTTP/API/实时接口。

## 预期目标

- 初始化 `./web` Bun 工程。
- 引入 React、TypeScript、Vite、Ant Design。
- 建立基础布局、路由、主题、暗色模式预留。
- 实现登录壳、Dashboard 骨架、Job 列表骨架、Instance 详情骨架。
- 建立 lint、typecheck、test、build 脚本。

## 验证命令

```bash
cd web
bun install
bun run lint
bun run typecheck
bun test
bun run build
```

同时后端仍需执行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
```

## 完成后更新

- `.memory/session-log.md`
- `.memory/progress.md`
- `.memory/commands.md`
- `.memory/next.md`
- 后续 `.prompt` 文件

验证通过后提交并推送。
