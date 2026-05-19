# 命令记录

当前仓库已初始化 Rust workspace。

预期基础命令：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config examples/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
```

若前端初始化在 `./web`：

```bash
cd web
bun install
bun run lint
bun run typecheck
bun test
bun run build
```


## 已验证命令（001-bootstrap）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config examples/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
```


## 已验证命令（002-http-api-and-openapi）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config examples/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json
curl -fsS http://127.0.0.1:9090/api/v1/system/info
curl -fsS http://127.0.0.1:9090/api/v1/cluster
curl -fsS http://127.0.0.1:9090/api/v1/jobs
curl -sS -o /tmp/create-job.json -w '%{http_code}' -H 'content-type: application/json' -d '{"name":"nightly"}' http://127.0.0.1:9090/api/v1/jobs
```


## HTTP 响应体契约检查

业务接口响应必须包含 `code`、`message`、`data`：

```bash
curl -fsS http://127.0.0.1:9090/api/v1/system/info
curl -fsS http://127.0.0.1:9090/api/v1/jobs
curl -sS -o /tmp/create-job.json -w '%{http_code}' -H 'content-type: application/json' -d '{"name":"nightly"}' http://127.0.0.1:9090/api/v1/jobs
```

Expected: success responses use `code=0`; failures use non-zero code; `data` key is always present.


## 已验证命令（003-worker-tunnel）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config examples/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json
# Smoke also verifies 127.0.0.1:9091 accepts TCP connection for Worker Tunnel gRPC listener.
```


## 已验证命令（004-storage-and-scheduler）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config examples/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json
curl -fsS http://127.0.0.1:9090/api/v1/jobs
curl -fsS -H 'content-type: application/json' -d '{"namespace":"default","app":"demo","name":"nightly","schedule_type":"api"}' http://127.0.0.1:9090/api/v1/jobs
curl -fsS http://127.0.0.1:9090/api/v1/jobs
```

说明：本阶段新增 SeaORM storage crate；SQLite dev DB 使用 `examples/dev.toml` 的 `sqlite://scheduler-dev.db?mode=rwc`。
