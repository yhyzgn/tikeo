# 002-http-api-and-openapi：HTTP 管理接口与 OpenAPI

## 阶段目标

在已完成的 Rust workspace 和 Axum healthz/readyz 骨架上，建立 `/api/v1` 管理面 API 基础、统一错误/分页模型和 OpenAPI 3.1 输出，为 Web UI、CLI、CI/CD 与 GitOps 接入打基础。

## 开始前必读

- `../prompt.md`
- `../design/tikeo-architecture-design.md` 中 5.5、5.6、13 章
- `../.memory/project.md`
- `../.memory/decisions.md`
- `../.memory/progress.md`
- `../.memory/commands.md`
- `../.memory/next.md`

## 当前代码上下文

- Cargo workspace 已初始化。
- 后端入口位于根 `src/main.rs`；Rust 模块 crate 均位于 `./crates/`：
  - `tikeo-core`
  - `tikeo-config`
  - `tikeo-server`
- `tikeo-server` 当前提供：
  - CLI: `tikeo serve --config config/dev.toml`
  - HTTP: `GET /healthz`、`GET /readyz`
- 依赖默认使用 Rust 1.95 兼容的最新稳定版。

## 硬性约束

- HTTP 业务接口响应必须统一为 `{code, message, data}`；`code=0` 表示成功，非 0 表示失败；`data` 即使为 null 也必须显式返回。

- 后端主程序入口保留在仓库根 `src/main.rs`；其余 Rust 代码继续放在 `./crates/*` 对应 crate 中。
- 不得把业务逻辑堆进 `tikeo-server`；DTO、错误、领域类型能抽到独立 crate 时优先抽离。
- HTTP API 不得直连 Worker；执行链路仍要预留 Worker Tunnel。
- 新增依赖默认使用当前最新稳定版；不能使用最新版时记录到 `.memory/decisions.md`。

## 建议任务

1. 选择 OpenAPI 生成库，优先评估 `utoipa` / `aide` / `schemars` 当前最新稳定版。
2. 在 `tikeo-server` 中建立 HTTP 模块分层：
   - `api` / `dto` / `error` / `pagination` / `routes`
   - 或更合适但同样解耦的结构。
3. 实现统一错误响应 Problem Details JSON：
   - `code`
   - `message`
   - `trace_id`
   - `details`
4. 实现基础 API：
   - `GET /api/v1/system/info`
   - `GET /api/v1/cluster`
   - `GET /api/v1/jobs` skeleton
   - `POST /api/v1/jobs` skeleton，可先返回 501 或受控占位，但 OpenAPI 要清楚表达。
5. 暴露：
   - `GET /api-docs/openapi.json`
   - `GET /api-docs/openapi.json`（仅 JSON，不提供文档 UI）。
6. 为新增 handler 和错误模型补测试。
7. 更新 `.memory` 和 `.prompt/003-worker-tunnel.md`。

## 验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/readyz
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
curl -fsS http://0.0.0.0:9090/api/v1/system/info
```

## 完成后必须更新

- `.memory/session-log.md`
- `.memory/progress.md`
- `.memory/commands.md`
- `.memory/next.md`
- `.memory/decisions.md`（如选择 OpenAPI 依赖）
- `.memory/risks.md`
- `.prompt/003-worker-tunnel.md`

验证通过后提交并推送。
