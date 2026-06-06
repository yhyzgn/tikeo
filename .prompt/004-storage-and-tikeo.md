# 004-storage-and-tikeo：存储层与基础调度器

## 阶段目标

在已完成的 HTTP/OpenAPI 与 Worker Tunnel 基础上，引入 SeaORM 存储层与最小 Job/Instance 模型，为 CRON/FIX_RATE/API 触发的调度链路做准备。

## 当前上下文

- 根 `src/main.rs` 是后端 binary entrypoint。
- Rust 模块位于 `./crates/*`。
- 已有 `tikeo-proto`，包含 Worker Tunnel proto/gRPC bindings。
- `tikeo-server` 已启动 HTTP `9090` 与 Worker Tunnel gRPC `9998`。
- HTTP API 统一 `{code,message,data}` envelope。

## 硬性约束

- HTTP 业务接口必须统一返回 `{code,message,data}`。
- 新 Rust 模块继续进入 `./crates/*`。
- 存储层应独立 crate，例如 `crates/tikeo-storage`。
- 依赖默认使用当前最新稳定版；不能使用最新版必须记录原因。
- 完成路线图项目后，必须在 `design/tikeo-architecture-design.md` 开发路线图中使用 `[x]` 标记，不额外添加 ✅ 图标。

## 建议任务

1. 新增 `crates/tikeo-storage`。
2. 引入 SeaORM / sea-orm-migration 当前最新稳定版。
3. 定义最小实体：namespace、app、job、job_instance。
4. 支持 SQLite dev database，配置项写入 `config/dev.toml`。
5. 为 `GET /api/v1/jobs` 接入真实 repository 查询。
6. 让 `POST /api/v1/jobs` 创建最小 job，而不是 501 placeholder。
7. 补充 migration 和 repository 测试。
8. 保持 HTTP/OpenAPI/Worker Tunnel 基础验证通过。

## 验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api/v1/jobs
```

完成后更新 `.memory/*`、后续 `.prompt`，提交并推送。
