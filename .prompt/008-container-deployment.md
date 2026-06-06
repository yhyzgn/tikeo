# 008-container-deployment：Docker / Compose / K8s 部署基础

## 阶段目标

在后端、Worker SDK、Java Starter 骨架和 Web 管理端基础工程已完成后，补齐容器化部署基础，确保 tikeo server 和未来 worker 可在不同容器、namespace 或集群中部署。

## 当前上下文

- 后端根 binary：`src/main.rs`，命令 `tikeo serve --config config/dev.toml`。
- Rust crate 位于 `./crates/*`。
- Web 工程位于 `./web`，React + TypeScript + Vite + Ant Design，Bun 管理。
- Worker Tunnel 默认监听 `0.0.0.0:9998`，Worker 必须主动出站连接。
- HTTP 管理 API 默认监听 `0.0.0.0:9090`。
- SQLite dev DB URL：`sqlite://tikeo-dev.db?mode=rwc`。

## 建议任务

1. 新增后端多阶段 `Dockerfile`，构建 Rust release binary。
2. 新增 Web Docker build 或将 Web 静态资源作为独立 nginx/caddy 容器服务。
3. 新增 `docker-compose.yml`，至少包含 tikeo server；可选包含 web 服务。
4. 新增 `deploy/k8s/` 基础 YAML：Deployment、Service、ConfigMap、PVC（SQLite dev only）或外部 DB 配置入口。
5. 明确 server 与 worker 跨网络部署：worker 只需访问 Worker Tunnel 服务地址，不暴露入站端口。
6. 更新 `config/` 配置，提供容器监听 `0.0.0.0` 示例。
7. 验证 Docker build / Compose 配置；若本机 Docker 不可用，记录验证缺口并至少完成配置静态检查。

## 验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
./sdks/java/gradlew -p sdks/java test
cd web
bun install
bun run lint
bun run typecheck
bun test
bun run build
```

如 Docker 可用：

```bash
docker build -t tikeo:dev .
docker compose config
docker compose up --build
```

完成后更新 `.memory/*`、后续 `.prompt`，提交并推送。
