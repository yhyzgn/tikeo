# 173 — 异常 demo、飞书卡片与公开执行控制台验收交接

## 本阶段已完成

- 各语言 Worker demo 已明确区分：
  - `demo.fail`：业务失败，返回 failed outcome。
  - `demo.exception`：程序运行时异常/抛错/panic/processor error，用于验证真实异常栈捕获。
- Node.js、Python、Go、Rust、Java/Spring SDK 均已在 processor exception 路径把异常栈写入 task log，并保持任务结果 failed。
- 飞书/Lark `interactive` 卡片模板已按截图风格实现：失败红色、成功绿色、普通/运行中蓝色，字段包含报警类型、运行环境、应用、任务 Handler、任务名称、触发时间、运行机器、执行结果、失败原因；底部按钮为“查看控制台”。
- provider metadata 为 Feishu interactive 暴露失败、成功、普通三套卡片示例，不再只有失败示例。
- Job notification payload 的 `logsUrl` / `consoleUrl` 均指向 `/public/instances/{id}/console`；生产环境可通过 `notification_delivery.public_console_base_url` 生成飞书/Lark 卡片可直接打开的绝对 URL。
- 后端公开 API `/api/v1/public/job-instances/{id}/trace` 已接入 router/OpenAPI；Web 公开路由 `/public/instances/:id/console` 位于 AuthGuard 外，API client 显式 `auth:false`。
- 公开 trace 页展示消息、策略、投递 attempts、实例上下文、脱敏最近日志摘要；日志仍做 password/token/secret/authorization/routingKey/signingKey 脱敏。
- `crates/tikeo-server/src/http/routes/notification_trace.rs` 已拆出，避免 `notifications.rs` 超过 1500 行。

## 已验证

- Rust workspace：`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --all-features -- -D warnings`、`cargo test --workspace --all-features`、`cargo build --workspace --all-features`。
- Web：`bun run --cwd web typecheck`、`bun run --cwd web lint`、`bun test --cwd web src`、`bun run --cwd web build`。
- Docs：`bun run --cwd docs docs:typecheck`、`bun run --cwd docs docs:build`。
- SDK 全量：Node/Python/Go/Rust/Java SDK tests。
- Demo：Node/Python/Go/Rust worker demo tests；Spring Boot 2/3/4 demo tests。
- Hygiene：`python3 scripts/check-source-size.py`、`git diff --check`。

## 后续注意

- 本地 `tikeo-dev.db` 因运行测试/本地服务有修改，不能提交。
- 公开执行控制台目前通过 notification message 反查实例 trace；若未来没有投递消息但仍希望公开查看，需要新增受控 token/link issuance，而不是放开所有实例日志。
- Live Feishu SaaS 验证仍需真实机器人 webhook/signing secret；本地只验证官方形态 JSON、渲染与本地 provider payload。生产推送时应设置 `TIKEO__NOTIFICATION_DELIVERY__PUBLIC_CONSOLE_BASE_URL`。
