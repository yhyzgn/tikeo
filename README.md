# scheduler

`scheduler` 是一个 Rust workspace 模式开发的分布式任务调度平台。后端主入口在根目录 `src/main.rs`，核心模块拆分在 `crates/*`；Web 管理端在 `web/`，使用 React + Ant Design + Bun。

## 本地开发一键启动

```bash
./scripts/dev.sh
```

脚本会自动：

1. 使用 `config/dev.toml` 启动后端 HTTP API 与 Worker Tunnel。
2. 如 `web/node_modules` 不存在，自动执行 `bun install`。
3. 启动 Web dev server，并通过 Vite proxy 访问后端 API。
4. 在 `.dev/server.log` 与 `.dev/web.log` 写入运行日志。

默认访问地址：

- Web UI: <http://127.0.0.1:5173>
- Backend API: <http://127.0.0.1:9090>
- OpenAPI JSON: <http://127.0.0.1:9090/api-docs/openapi.json>

> 项目不提供浏览器 API 文档 UI；仅保留机器可读的 OpenAPI JSON。

## 初始化专用账号

开发周期内置一组初始化专用账号，便于直接登录 Web UI 调试：

| 字段 | 默认值 |
| --- | --- |
| 用户名 | `scheduler_init` |
| 密码 | `Scheduler@2026!` |

可通过环境变量覆盖：

```bash
export SCHEDULER_DEV_ADMIN_USERNAME="scheduler_init"
export SCHEDULER_DEV_ADMIN_PASSWORD="Scheduler@2026!"
./scripts/dev.sh
```

当前账号仅用于开发初始化登录调试；系统不再内置静态 Bearer 后门，所有受保护 API 都必须先通过登录接口获取 `atk_` 会话 token。生产阶段必须接入正式 RBAC / OIDC / API Token 生命周期管理。

## 配置目录

配置文件统一放在 `config/`：

- `config/dev.toml`：本地开发配置，监听 `127.0.0.1:9090` / `127.0.0.1:9091`。
- `config/container.toml`：容器部署配置，监听 `0.0.0.0:9090` / `0.0.0.0:9091`。

## 常用验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
```

## Docker / Compose

```bash
DOCKER_BUILDKIT=1 docker build -t scheduler:dev .
DOCKER_BUILDKIT=1 docker build -t scheduler-web:dev ./web
docker compose up -d --no-build
```

Docker/Compose 验证必须使用默认 bridge 网络，不使用 host 网络规避真实网络层问题。
