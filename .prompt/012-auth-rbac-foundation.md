# 012-auth-rbac-foundation：登录与权限感知操作基础

## 阶段目标

在任务创建、API/CRON/Fixed Rate 触发、Worker 执行回传和实例日志查看基础完成后，补齐平台管理端的最小认证与权限感知基础，为后续企业 RBAC / OIDC / 审计打底。

## 当前上下文

- 后端 root binary 在 `src/main.rs`；业务模块在 `crates/*`。
- HTTP API 必须返回 `{ code, message, data }`，`code=0` 成功，`data` 必须显式存在。
- Web 在 `web/`，React + Ant Design + Bun。
- Worker 仍只能主动出站连接 Worker Tunnel；认证工作不应改变 Worker 网络模型。
- 目前 Web 缺少登录与权限感知操作；Phase 1 路线图中该项仍未完成。

## 建议任务

1. 设计最小 auth 数据/配置模型：开发态 admin 用户或静态 token；明确生产 OIDC/RBAC 扩展点。
2. 后端新增 auth API：login/me/logout 或 token 校验端点；保持统一 envelope。
3. 为管理 API 增加可测试的认证中间件或占位策略，避免破坏 health/openapi。
4. Web 增加登录页、登录态保存、退出入口和 API client token 注入。
5. Job trigger/create 等操作体现权限感知；未登录时不允许操作。
6. 增加 Rust HTTP tests 和 Web tests。
7. 更新设计路线图、`.memory/*`、后续 `.prompt`，全量验证后提交并推送。

## 验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f java/pom.xml -q test
bun install --cwd web
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
docker build -t scheduler:dev .
docker build -t scheduler-web:dev ./web
```

完成后必须更新路线图、记忆库、后续 prompt，提交并推送。
