---
title: 安装
description: Tikeo 的工具链矩阵、版本基线、仓库工程面、首次初始化 Owner 前置条件和验证命令。
---

# 安装

本页用于准备真实的 Tikeo 评估环境。它不只说明如何 clone 仓库，还说明每个工具链服务哪个工程面、哪些命令能证明这个工程面可用、首次 Owner 初始化在哪里发生，以及哪些失败通常说明环境还没准备好。

## 工具链矩阵

| 工程面 | 目录 | 工具 | 作用 |
| --- | --- | --- | --- |
| Server/Rust crates | 根目录、`crates/*`、`src/main.rs` | Rust 1.95+、Cargo | 构建 `tikeo` 二进制、迁移、Worker Tunnel、HTTP API 和核心测试。 |
| Web 控制台 | `web/` | Bun、Node 兼容运行时 | 构建 React/TypeScript/Ant Design 运维控制台。仓库命令必须用 Bun。 |
| Docs 站点 | `docs/` | Bun | 构建 Docusaurus 文档站和 docs Docker 镜像。 |
| Java SDK/demo | `sdks/java`、`examples/java/*` | Java 17+ runtime、Gradle 配置的 JDK toolchain | 构建 Java core SDK、Spring 模块、Boot 2/3/4 starter 和 Worker demo。 |
| Go SDK/demo | `sdks/go/tikeo`、`examples/go/worker-demo` | Go 1.26+ | 构建 Go SDK、Management client 和 Worker demo。 |
| Python SDK/demo | `sdks/python/tikeo`、`examples/python/worker-demo` | Python 3.11+ | 构建 Python 包、Worker client、脚本 helper 和测试。 |
| Node.js SDK/demo | `sdks/nodejs/tikeo`、`examples/nodejs/worker-demo` | Bun；消费者基线 Node.js 24+ | 构建 TypeScript SDK、demo Worker、Management client 和 dist。 |
| 容器 | 根 `Dockerfile`、`web/Dockerfile`、`docs/Dockerfile` | Docker + BuildKit | 构建 Server/Web/Docs 镜像。 |
| Kubernetes | `deploy/helm/tikeo`、`deploy/k8s` | `kubectl`、`helm` | 安装 Server/Web。业务 Worker 仍然出站连接。 |

## 版本基线

README badge 只提供概览，本地开发以仓库文件为准：根 Cargo workspace 使用 Rust 2024 edition 和 `Cargo.lock`；`web/package.json` 与 `docs/package.json` 都是 Bun 驱动模块；Java 模块在 `sdks/java/settings.gradle.kts` 中声明；Python 要求来自 `sdks/python/tikeo/pyproject.toml` 的 `>=3.11`；Node 包在 `sdks/nodejs/tikeo/package.json` 中声明 `@yhyzgn/tikeo` 和 Node.js `>=24.0.0`；Server 配置默认值来自 `crates/tikeo-config/src/lib.rs`，不是从示例文件猜出来。

## 克隆并认识仓库工程面

```bash
git clone https://github.com/yhyzgn/tikeo.git
cd tikeo
find . -maxdepth 2 -type d | sort | sed -n '1,80p'
```

关键目录：`config/` 是 Server YAML 示例；`crates/` 是 Rust library crates；`src/main.rs` 是二进制入口；`web/` 是控制台；`docs/` 是文档站；`sdks/` 是语言 SDK；`examples/` 是 Worker demo；`deploy/` 是 Compose/Helm/K8s/systemd/Terraform/smoke；`scripts/` 是本地开发和验收脚本；`.github/tests/` 是契约测试。

## 验证工具链

```bash
rustc --version
cargo --version
bun --version
docker --version || true
go version || true
java -version || true
python3 --version || python --version
```

只评估 Server + Node.js Worker 时不需要所有语言工具；要跑完整跨语言 smoke 时才需要全部安装。

## 按模块安装依赖

```bash
cargo fetch
cargo test --workspace --all-features --no-run
(cd web && bun install --frozen-lockfile)
(cd docs && bun install --frozen-lockfile)
(cd sdks/nodejs/tikeo && bun install --frozen-lockfile)
(cd examples/nodejs/worker-demo && bun install --frozen-lockfile)
(cd sdks/python/tikeo && python3 -m pip install -e '.[test]')
(cd sdks/go/tikeo && go test ./... -count=1)
./sdks/java/gradlew -p sdks/java test --no-daemon
```

## 首次初始化 Owner

启动 Server 只证明 listener 和迁移。要作为人类操作者使用 Web/API，必须对当前数据库初始化一次 Owner。检查状态：

```bash
curl -fsS http://127.0.0.1:9090/api/v1/auth/bootstrap | jq .
```

本地一次性注册：

```bash
BOOTSTRAP_USERNAME="${TIKEO_BOOTSTRAP_USERNAME:-owner-$(date +%s)}"
BOOTSTRAP_EMAIL="${TIKEO_BOOTSTRAP_EMAIL:-${BOOTSTRAP_USERNAME}@example.invalid}"
BOOTSTRAP_PASSWORD="${TIKEO_BOOTSTRAP_PASSWORD:-$(openssl rand -base64 24 | tr -d '\n')}"
jq -n \
  --arg username "$BOOTSTRAP_USERNAME" \
  --arg email "$BOOTSTRAP_EMAIL" \
  --arg password "$BOOTSTRAP_PASSWORD" \
  '{username:$username,email:$email,password:$password,confirmPassword:$password}' \
  | curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/bootstrap/register \
      -H 'content-type: application/json' \
      -d @- | jq .
```

后续登录：

```bash
jq -n \
  --arg username "$TIKEO_BOOTSTRAP_USERNAME" \
  --arg password "$TIKEO_BOOTSTRAP_PASSWORD" \
  '{username:$username,password:$password}' \
  | curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/login \
      -H 'content-type: application/json' \
      -d @- | jq .data.token
```

这些示例凭证只能用于隔离本地 DB。CI smoke 会在 `.dev/reports/...` 下创建自己的临时 DB 和临时凭证。

## 推荐验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
python3 scripts/check-source-size.py
python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24
(cd web && bun run typecheck && bun run test && bun run build)
(cd docs && bun run docs:typecheck && bun run docs:build)
docker build -f docs/Dockerfile docs -t tikeo-docs:local
python3 .github/tests/docs_site_contract_test.py
python3 .github/tests/workflow_contract_test.py
python3 .github/tests/management_smoke_contract_test.py
```

## 最小本地启动

```bash
cargo run --bin tikeo -- serve --config config/dev.yml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

`config/dev.yml` 绑定 HTTP `0.0.0.0:9090`、Worker Tunnel `0.0.0.0:9998`，SQLite 是 `.dev/tikeo-dev.db`，时间偏移是 `+00:00`。跨环境比较时间时要读配置参考。

## 常见安装错误

| 现象 | 常见原因 | 处理 |
| --- | --- | --- |
| `bun install --frozen-lockfile` 失败 | 包管理器不对、registry 漂移、网络认证问题 | 用 Bun，并确认 lockfile 不指向私有 registry，除非明确配置。 |
| Worker 连接失败 | Server 没有监听 `9998`、endpoint 错、TLS/plaintext 不匹配 | 先启动 Server，本地用 `TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998`。 |
| API unauthorized | Owner 未初始化或 token 缺失 | 初始化 Owner，登录后给 human route 传 bearer token。 |
| SDK Management unauthorized | 应用级 SDK API Key 缺失或无权限 | 创建 service account + API key，用 `x-tikeo-api-key`。 |
| SQLite 锁冲突 | 多个进程共用同一 DB | smoke 使用隔离 DB，本地清理旧进程。 |
| Helm 示例本地失败 | 缺少 helm、CRD 或 controller | 本地先用 `helm template`/契约测试，live controller smoke 只在安装了 controller 的集群跑。 |

## 下一步

继续 [快速开始](./quickstart)，完成 Server + Web + Worker + SDK Management API 端到端验收；如果只查配置项，跳到 [配置参考](../reference/configuration)。

## 安装验收标准

安装完成不是“命令能敲出来”，而是至少能证明三个事实。第一，Rust Server 能编译并运行迁移；第二，Web 与 Docs 两个 Bun 模块能独立 typecheck/build；第三，至少一个 Worker SDK demo 能在 dry-run 或 live 模式下展示结构化 capability。对于只做文档或部署工作的贡献者，也要运行 docs contract 和 workflow contract，避免把不存在的配置项、路径或镜像写进文档。

如果准备把环境交给其他人复现，请记录这些输出：`cargo --version`、`bun --version`、`docker --version`、Server `readyz` 响应、Web build 结果、docs build 结果，以及 smoke 生成的 `.dev/reports/...` 路径。没有这些证据，就不要说“环境已经搭好”。

## 与生产部署的差异

本地安装默认追求快速反馈：SQLite 文件在仓库目录，HTTP 和 Worker Tunnel 都是 plaintext，OIDC 关闭，日志主要走控制台，Worker endpoint 是 `127.0.0.1:9998`。生产或共享环境通常要改成外部 PostgreSQL/MySQL、Secret 注入 `TIKEO__STORAGE__DATABASE__HOST / TIKEO__STORAGE__DATABASE__PASSWORD`、Ingress 或进程内 TLS、Worker Tunnel TLS/mTLS、持久日志目录、OTel collector、明确的 service account 和短权限 SDK API Key。

不要把本地 `bootstrap_admin` 示例密码、demo API key、或 demo namespace 当作生产约定。它们只是为了让读者在隔离 SQLite DB 上复现流程。生产 runbook 应该从配置参考和部署页面重新选择值，并保留 smoke/CI 证据。

## 前置条件

执行本页命令前，请先满足页面列出的安装、认证和权限要求。本地示例默认 Server 使用 `config/dev.yml`，客户端访问 `127.0.0.1`，令牌保存在 shell 变量中，不写入文件或截图。

## 验收

完成本页步骤后，用对应 API、UI、构建、smoke 或部署检查验证结果。有效验收至少包含执行的命令、检查的路由或文件，以及观察到的状态或产物。

## 故障排查

步骤失败时，先保留完整命令、响应状态和 Server 日志时间窗口，再检查认证、namespace/app scope、Worker 匹配、存储 readiness 和代理行为，不要直接修改生产配置。

## 生产检查清单

- [ ] 密钥通过环境变量或平台 Secret 引用管理，不写入示例。
- [ ] 已把本地 `127.0.0.1` 命令替换成真实域名、TLS 和认证方式。
- [ ] 已记录变更面的回滚和证据采集方式。
- [ ] 运维人员可以在没有隐藏 shell 历史或隐式状态的情况下复现验收。
