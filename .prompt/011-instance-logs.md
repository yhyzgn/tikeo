# 011-instance-logs：实例执行日志与 Web 查看基础

## 阶段目标

在 010 已完成 API / CRON / Fixed Rate 触发、Worker Tunnel dispatch 和 Rust SDK TaskProcessor 回传后，补齐实例执行日志的最小闭环：Worker 通过主动建立的 tunnel 回传日志，Server 持久化，HTTP API 与 Web UI 可以查询展示。

## 当前上下文

- Root binary 入口在 `src/main.rs`，后端模块在 `crates/*`。
- HTTP API 必须返回 `{ code, message, data }`，`data` 即使为 null 也必须存在。
- Worker 仍只能主动出站连接 `OpenTunnel`；不得新增 Worker 入站端口。
- `job_instances` 已支持 pending/running/succeeded/failed 状态流转。
- `scheduler-server::scheduler` 已支持 cron/fixed_rate 自动创建 pending instance。
- Web 位于 `web/`，React + Ant Design + Bun，Instances 页面已有基础列表。

## 建议任务

1. Storage：新增 `job_instance_logs` 表与 repository：
   - id、instance_id、worker_id、level、message、sequence、created_at。
   - 支持 append 与按 instance 分页查询。
2. Proto / Tunnel：新增 Worker -> Server `TaskLog` 消息。
3. Rust SDK：提供 `TaskContext` 日志上报接口或 `WorkerSession::emit_log` 最小方法。
4. HTTP API：新增 `GET /api/v1/instances/{instance}/logs`，返回统一 envelope。
5. Web UI：Instances 页面增加日志查看入口（Drawer/Modal/Table 均可），调用日志 API 展示。
6. 测试：storage append/list、tunnel log persistence、API envelope、Web client 测试。
7. 更新设计路线图、`.memory/*`、后续 `.prompt`，全量验证后提交并推送。

## 验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
./sdks/java/gradlew -p sdks/java test
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
