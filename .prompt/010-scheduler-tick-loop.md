# 010-scheduler-tick-loop：CRON / Fixed Rate 自动触发

## 阶段目标

在 009 已打通 Worker Tunnel dispatch -> Rust SDK TaskProcessor -> TaskResult 状态回传后，实现基础调度 tick loop，让 `cron` 与 `fixed_rate` Job 可以不依赖手动 API trigger 自动创建 job_instance，并复用现有 dispatch loop 执行。

## 当前上下文

- Root binary 入口仍在 `src/main.rs`；业务模块在 `crates/*`。
- HTTP API 必须统一返回 `{ code, message, data }`，`code=0` 成功，`data` 必须显式存在。
- `scheduler-storage` 已支持 Job/JobInstance 持久化、pending 查询、status update。
- `scheduler-server::tunnel::dispatcher` 已定期分发 pending instance 到 first available Worker。
- `scheduler-worker-sdk::WorkerSession::process_next` 已能接收 dispatch 并回传结果。
- 当前 `ScheduleType` 支持 `api`、`cron`、`fixed_rate`、`fixed_delay`；但 CRON / Fixed Rate tick loop 尚未实现。

## 建议任务

1. 在 storage 层补充查询 enabled scheduled jobs 的 repository 方法。
2. 设计最小 schedule cursor：
   - 可先内存记录 last trigger timestamp，后续再持久化。
   - 避免同一 tick 重复创建实例。
3. 实现 scheduler tick loop：
   - cron：选用稳定 Rust cron 解析库或最小表达式支持；依赖需使用最新稳定版。
   - fixed_rate：按毫秒/秒级表达式或明确格式触发。
4. 自动触发时创建 pending `job_instance`，trigger_type 分别为 `cron` / `fixed_rate`。
5. 与 dispatch loop 并行运行，确保自动实例可被 Worker Tunnel 分发。
6. 增加单元测试：
   - cron/fixed_rate job 到期创建实例。
   - disabled job 不触发。
   - 同一 tick 不重复触发。
7. 更新设计路线图、`.memory/*`、后续 `.prompt`。

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
