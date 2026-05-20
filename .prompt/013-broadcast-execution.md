# 013-broadcast-execution

## 背景

当前已完成 012-auth-rbac-foundation：
- 后端提供开发管理员登录 `/api/v1/auth/login`、当前身份 `/api/v1/auth/me`、退出 `/api/v1/auth/logout`。
- 写操作 `POST /api/v1/jobs` 与 `POST /api/v1/jobs/{job}:trigger` 已要求 `Authorization: Bearer <token>`。
- Web 已提供登录页、token 持久化、退出入口，并对创建/触发写操作自动携带 bearer token。
- 所有 HTTP 业务接口仍必须保持 `{ code, message, data }` envelope，`data` 字段必须出现。
- Worker 仍必须只通过主动发起的 `OpenTunnel` 通道接收任务与回传结果，不能打开入站端口。

## 目标

实现 Phase 1 中“广播执行”的第一版基础能力，让一个 Job 可以触发多个 Worker 各执行一次，并能在实例/子执行维度观察结果。

## 技术约束

- Rust backend：workspace 模式；业务模块继续放在 `crates/*`，根主程序入口保持 `src/main.rs`。
- Web：`web/`，React + Ant Design + Bun。
- 不破坏已有单机执行链路。
- 不引入 Worker 入站调用；Server 只能经 Worker 主动建立的 Tunnel 下发任务。
- API 返回必须统一 `{ code, message, data }`。
- 完成后更新 `.memory/*`、本文件之后的下一阶段 `.prompt/014-*.md`、以及 `design/scheduler-architecture-design.md` 路线图 `[x]` 标记。

## 建议实现范围

1. 领域与存储
   - 明确 broadcast 的触发语义：初版可在 trigger request 中增加 `execution_mode: single|broadcast`，默认 single。
   - 增加必要的 storage 表/字段以记录一次广播下的多个 Worker 执行结果；若当前 `job_instances` 不足以表达子执行，新增 `job_instance_attempts` 或类似表。
2. Tunnel dispatch
   - registry 暴露当前可用 Worker 列表。
   - broadcast 触发时向所有符合条件的在线 Worker 下发任务。
   - 汇总状态：所有子执行成功则 succeeded；任一失败可标记 partial_failed/failed（需要在 core 中补充稳定状态值）。
3. HTTP/OpenAPI
   - 扩展 trigger request/response schema。
   - 增加查询广播子执行结果的接口（如 `/api/v1/instances/{instance}/attempts`）。
4. Web
   - Job 手动触发支持选择 single/broadcast。
   - Instance 详情或列表可查看广播子执行结果。
5. 测试
   - 覆盖 single 默认兼容。
   - 覆盖两个 Worker 在线时 broadcast 下发两份任务并汇总结果。

## 验证要求

必须至少运行并通过：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
docker build -t scheduler:dev .
docker build -t scheduler-web:dev ./web
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080
docker compose down
```

提交时使用丰富且有层次的 Lore-style commit message，并推送远程 `main`。
