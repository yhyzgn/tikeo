# 014-worker-capability-routing

## 背景

013-broadcast-execution 已完成基础广播执行：
- `TriggerJobRequest.execution_mode` 支持 `single` / `broadcast`，默认 single。
- 广播触发会基于当前在线 Worker 创建 `job_instance_attempts` 子执行记录。
- Dispatcher 会通过 Worker 主动建立的 OpenTunnel 向指定 Worker 下发广播 attempt；Worker 无入站端口。
- `TaskResult` 会更新子执行并汇总父实例状态（全部成功 -> succeeded，部分失败 -> partial_failed）。
- HTTP 新增 `GET /api/v1/instances/{instance}/attempts`；Web 可选择广播触发并查看子执行。

## 目标

实现 Worker 能力 / 标签 / namespace / app 的基础路由，让 single 与 broadcast 都只发送给符合条件的 Worker。

## 约束

- Rust backend 保持 workspace，根入口仍在 `src/main.rs`。
- Web 保持 `web/` React + Ant Design + Bun。
- Worker 仍只能主动 OpenTunnel；Server 不得直接反向连接 Worker。
- HTTP API 必须保持 `{code,message,data}` envelope。
- 完成后更新 `.memory/*`、`design/scheduler-architecture-design.md`、并新增 `.prompt/015-*.md`。

## 建议范围

1. Job 路由需求
   - 在 Job 或 Trigger request 中增加 worker selector：namespace/app 默认匹配，后续可扩展 labels/capabilities。
   - 先实现基础规则：Worker namespace/app 与 Job namespace/app 匹配；允许 `*` 或空值作为通配。
2. Registry
   - 增加按 selector 查询 Worker 的方法。
   - 保持已有 `worker_ids` / `dispatch_to_worker` 兼容。
3. Dispatcher
   - single：选择第一个符合条件的 Worker。
   - broadcast：只为符合条件的 Worker 创建 attempt / 或只向符合条件 Worker 下发。
4. HTTP/OpenAPI/Web
   - Web 展示当前路由约束说明；必要时触发广播失败要给出清晰错误。
5. 测试
   - 两个 Worker 分属不同 app 时，single/broadcast 只命中目标 app。
   - 无符合 Worker 时返回稳定 envelope 或保持 queued 的行为需要明确并测试。

## 验证要求

至少运行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
./sdks/java/gradlew -p sdks/java test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
DOCKER_BUILDKIT=1 docker build -t scheduler:dev .
DOCKER_BUILDKIT=1 docker build -t scheduler-web:dev ./web
docker compose down --remove-orphans || true
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080
curl -fsS http://0.0.0.0:8080/api/v1/system/info
curl -fsS http://0.0.0.0:8080/api-docs/openapi.json
docker compose down
```

提交并推送到远程 `main`，commit message 保持 Lore-style。
