# 命令记录

当前仓库尚未初始化 Rust workspace。代码开发阶段建立 workspace 后，必须补充实际命令。

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
