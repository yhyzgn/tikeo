# 003-worker-tunnel：Worker 主动连接与注册心跳

## 阶段目标

在当前 root binary + `crates/*` 解耦结构上，引入最小 gRPC/protobuf Worker Tunnel：Worker 主动连接 Server、注册、心跳，Server 维护连接路由表雏形。该阶段只做最小可验证切片，不实现完整任务分发。

## 开始前必读

- `../prompt.md`
- `../design/scheduler-architecture-design.md` 中 Worker Tunnel、部署、通信协议章节
- `../.memory/project.md`
- `../.memory/decisions.md`
- `../.memory/progress.md`
- `../.memory/commands.md`
- `../.memory/next.md`

## 当前代码上下文

- 后端主程序入口位于根 `src/main.rs`，只负责调用 `scheduler_server::run_cli()`。
- 业务/平台模块位于 `./crates/*`：
  - `scheduler-core`
  - `scheduler-config`
  - `scheduler-server`
- HTTP 管理 API 已具备：
  - `/healthz`
  - `/readyz`
  - `/api-docs/openapi.json`
  - `/api/v1/system/info`
  - `/api/v1/cluster`
  - `/api/v1/jobs` GET/POST skeleton
- OpenAPI 依赖已选择：`utoipa`；禁止 API 文档 UI 依赖。

## 硬性约束

- HTTP 业务接口响应必须统一为 `{code, message, data}`；`code=0` 表示成功，非 0 表示失败；`data` 即使为 null 也必须显式返回。

- 后端入口继续保留在根 `src/main.rs`。
- 根 `src/` 不承载业务模块，业务能力必须进入 `crates/*`。
- Worker 不暴露入站端口。
- Server 不直连 Worker。
- Server→Worker 指令必须复用 Worker 主动建立的双向流。
- Worker identity 以 app、namespace、cluster、region、labels、capabilities 逻辑寻址。
- 新依赖默认使用当前最新稳定版；不能使用最新版时记录到 `.memory/decisions.md`。

## 建议任务

1. 新增 `crates/scheduler-proto` 或同等 proto crate。
2. 引入当前最新稳定的 `tonic` / `prost` / `tonic-build` 方案。
3. 创建 `proto/scheduler/worker/v1/worker.proto`，定义最小消息：
   - `WorkerMessage`
   - `ServerMessage`
   - `RegisterWorker`
   - `Heartbeat`
   - `WorkerRegistered`
4. 在 server 侧实现 `WorkerTunnelService::Connect(stream WorkerMessage) -> stream ServerMessage` 最小版本。
5. 增加内存连接 registry 雏形，至少记录 worker id、app、最后心跳时间。
6. 增加单元测试或集成测试，验证注册/心跳消息能被处理。
7. 保持现有 HTTP/OpenAPI 冒烟全部通过。

## 验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config examples/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json
```

如增加 gRPC 端口或同端口 multiplex 方案，必须补充对应 smoke test。

## 完成后更新

- `.memory/session-log.md`
- `.memory/progress.md`
- `.memory/commands.md`
- `.memory/next.md`
- `.memory/decisions.md`
- `.memory/risks.md`
- `.prompt/004-storage-and-scheduler.md`

验证通过后提交并推送。
