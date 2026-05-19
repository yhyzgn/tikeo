# 001-bootstrap：Rust 工程骨架初始化

## 阶段目标

建立 scheduler 的最小 Rust workspace（所有 crate 位于 `./crates/`）与可运行服务骨架，为后续 HTTP API、Worker Tunnel、存储和 Web UI 开发提供稳定基础。

## 开始前必读

- `../prompt.md`
- `../design/scheduler-architecture-design.md`
- `../.memory/project.md`
- `../.memory/progress.md`
- `../.memory/decisions.md`
- `../.memory/commands.md`
- `../.memory/risks.md`

## 设计依据摘要

- Rust 2024 Edition。
- Server 单二进制，未来内置 Web UI。
- HTTP 框架使用 Axum。
- CLI 使用 Clap。
- 配置使用 config-rs / serde。
- 日志使用 tracing。
- 后续需要支持 gRPC、SeaORM、OpenAPI、Worker Tunnel。

## 依赖策略

- 新增 Rust crate 时优先使用当前最新稳定版。
- 若不能使用最新版，必须在 `.memory/decisions.md` 记录原因、锁定版本和升级条件。
- Bootstrap 阶段应尽量加入基础依赖安全检查工具或在 `.memory/risks.md` 记录暂缓原因。

## 任务列表

1. 初始化 Cargo workspace，根目录只放 workspace 配置，不承载业务代码。
2. 在 `./crates/` 下创建建议 crate（所有 Rust 代码模块必须以 crate 形式解耦）：
   - `crates/scheduler-server`：server binary / HTTP gateway / CLI serve。
   - `crates/scheduler-core`：领域模型与核心 trait 占位。
   - `crates/scheduler-config`：配置加载。
   - 后续可增加 `scheduler-proto`、`scheduler-storage`、`scheduler-worker-sdk`。
3. 增加根 `Cargo.toml` workspace 配置。
4. 增加 `rustfmt.toml`、基础 `.gitignore`，并确保 workspace members 只指向 `crates/*` 下 crate。
5. 实现 `scheduler serve`：
   - 读取默认配置。
   - 启动 Axum HTTP server。
   - 暴露 `GET /healthz` 与 `GET /readyz`。
6. 增加最小测试：
   - 配置加载测试。
   - healthz handler 测试。
7. 增加 `examples/dev.toml`。
8. 记录 Rust workspace 的实际命令；若创建前端占位，只能创建 `./web/`，不得创建 `webui/`。
9. 更新 `.memory/commands.md` 中实际命令。
10. 创建或更新 `.prompt/002-http-api-and-openapi.md`。

## 验证命令

必须执行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
```

冒烟运行：

```bash
cargo run --bin scheduler -- serve --config examples/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
```

如果端口或命令调整，必须更新 `.memory/commands.md`。

## 完成后必须更新

- `.memory/session-log.md`
- `.memory/progress.md`
- `.memory/commands.md`
- `.memory/next.md`
- `.memory/risks.md`
- `.prompt/002-http-api-and-openapi.md`

## 提交与推送

验证通过后提交并推送。提交信息需要包含：

- 为什么初始化该骨架
- crate 划分
- 已验证命令
- 冒烟结果
- 下一阶段

示例首行：

```text
🚀 奠定 scheduler Rust 开发骨架以支撑平台化调度能力
```
