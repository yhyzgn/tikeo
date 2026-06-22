---
title: Tikeo 是什么？
slug: /
description: Tikeo 是 Rust 原生任务调度、工作流 DAG、出站 Worker Tunnel、多语言 SDK、脚本治理、RBAC、审计和部署自动化平台。
---

# Tikeo 是什么？

Tikeo 是一个 Rust 原生的任务编排控制平面，目标不是只提供一个定时器，而是把计划任务、API 触发任务、工作流 DAG、出站 Worker Tunnel、多语言 SDK Processor、受治理脚本、通知中心投递、告警边界、RBAC、审计证据、Web 运维控制台、Docker/Helm/Terraform 部署资产和可验证示例连接成一个系统。

README 只负责“为什么值得看”和“十分钟入口”。文档站负责“能不能照着搭起来、接入 SDK、部署、排错、验收”。阅读结果应该非常具体：你能安装工具链，启动 Server，创建首个 Owner，创建 namespace/app/worker pool，创建应用级 SDK API Key，让 Worker 通过 Worker Tunnel 主动出站连接，用 SDK 创建并触发任务，检查实例、日志和审计证据，并知道从本地 SQLite 迁到 PostgreSQL/MySQL、Compose 或 Helm 时哪些默认值发生了变化。

## 文档地图

| 阶段 | 页面 | 你会得到什么 |
| --- | --- | --- |
| 1 | [安装](./getting-started/installation) | 工具链矩阵、版本基线、仓库工程面、构建/测试命令、首次初始化 Owner 的前置条件。 |
| 2 | [快速开始](./getting-started/quickstart) | 本地 Server + Web + Worker + SDK Management API 的可验收路径。 |
| 3 | [配置参考](./reference/configuration) | 完整默认值表、环境变量覆盖名、示例文件、TLS/mTLS、OIDC、日志、OTel、集群注意事项、Worker SDK 默认值。 |
| 4 | [Worker Tunnel](./concepts/worker-tunnel) | Worker 为什么主动出站连接，注册携带什么，为什么不能创建业务 Worker 入站 Service。 |
| 5 | SDK 页面 | 依赖坐标、WorkerConfig 默认值、最小 Worker、管理客户端凭证、现场验收 runbook。 |
| 6 | 部署页面 | 单二进制、Docker Compose、Kubernetes/Helm、[Server 高可用与 Raft FSOD 集群](./deployment/server-ha)、控制器差异和 smoke 脚本。 |
| 7 | 参考页面 | 可验收的 Management OpenAPI、通知中心和 Worker Tunnel protobuf 参考。 |
| 8 | [产品就绪验收清单](./development/product-readiness-acceptance) | 通知中心、旧调度器迁移 CLI 和 Raft FSOD Server HA 的跨功能发布/交接门槛。 |
| 9 | [v0.3.9 发布验收包](./development/release-acceptance-packet-v0.3.9) | 具体 release/交接证据：assets、CI、Kind HA 指标、跨语言 Worker soak 和剩余生产检查。 |

如果只想证明本地全链路仍然可用，运行快速开始里的 Management trigger smoke。如果要写生产 runbook，先读配置和部署，再选择服务团队要用的 SDK 页面。

## 架构边界

Tikeo 的核心边界必须保持清晰：Server 负责调度、治理、持久化、审计和派发；Worker 负责执行。Server 不执行用户业务代码。业务 Worker 不需要入站端口。Worker 主动连接 gRPC/HTTP2 Worker Tunnel，发送结构化 capability，收到 Server 分配的权威 `worker_id`，带 lease/fencing 元数据心跳，接收 `DispatchTask`，发送 `TaskLog`，并用 Server 下发的 assignment token 返回 `TaskResult`。

这条边界直接影响部署和排障。Worker 可以在私有 namespace、客户 VPC、NAT 后面、另一朵云或 VM 上，只要能访问 Worker Tunnel endpoint。不要为了让调度器能调用 Worker 而暴露业务 Worker HTTP 服务。因此 Helm chart 只安装 Server 和 Web；业务 Worker 应作为独立 Deployment、DaemonSet、sidecar、VM/systemd 服务或嵌入式 SDK 客户端出站连接。

## 仓库中已经存在的工程面

- Rust workspace：配置、存储、服务端、协议和 WASM 边界。
- 单一 `tikeo` 二进制，支持 `serve --config <path>` 和 `TIKEO_CONFIG`。
- `config/` 中提供 dev/container/PostgreSQL/MySQL/raft shape 示例。
- `web/` 是 React + TypeScript + Vite + Ant Design + Bun 的控制台。
- `docs/` 是 Docusaurus 文档站模块，并可构建 docs Docker 镜像。
- Rust、Go、Java/Spring Boot、Python、Node.js Worker SDK。
- `examples/<language>/worker-demo` 中可运行 Worker demo，带结构化 processor capability。
- SDK Management helper 可用应用级 `x-tikeo-api-key` 创建 API job 并触发。
- Docker Compose、Helm、Kubernetes YAML、systemd、Terraform、GitOps diff 和 smoke 脚本。
- `.github/tests/*` 防止文档、workflow、部署、通知中心索引和 smoke 表面漂移。

页面中的命令必须能对应到这些工程面。如果功能尚不适合生产，文档要明确写出限制；实验性能力必须明确标注，不应被描述成生产 runbook。

## 阅读结果

本文档站服务四类读者：平台评估者、应用工程师、SRE/平台运维、贡献者。平台评估者要判断 Tikeo 是否能替代传统调度器，尤其是 Worker 不能开放入站端口的场景。应用工程师要知道如何添加 SDK 依赖、声明 processor、连接 Worker Tunnel、从应用级凭证触发任务。SRE 要知道如何部署 Server/Web、配置存储、TLS/mTLS、日志、OTel、ingress/gateway、备份、回滚和 smoke。贡献者要知道如何运行测试、让文档保持可验证、避免发明端点，并在工作后更新 project handoff 交接。

因此这里优先使用表格、默认值、可复制命令、预期观察和失败排查，而不是营销段落。

## 证据优先验收

本地验收至少应该产生多层证据：

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
cd web && bun install --frozen-lockfile && bun run typecheck && bun run build
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

smoke 脚本比截图更强，因为它会启动隔离 Server，在 `.dev/reports/management-trigger-e2e-*` 下写入独立 SQLite 配置和 DB，初始化 scope，创建 service account 和 SDK API key，用 `TIKEO_WORKER_CONNECT=1` 启动 Node.js Worker demo，通过 SDK Management client 创建并触发任务，最后记录实例结果和日志证据。

## operator-verified 规则

文档不得发明 API 名称、包坐标、配置项或部署参数。主要证据源包括 `crates/tikeo-config/src/lib.rs`、HTTP router/OpenAPI/route 文件、`crates/tikeo-proto/proto/worker.proto`、`sdks/*`、`examples/*`、`deploy/*`、Dockerfile、workflow 和 smoke 脚本。通知中心与告警页面还必须核对 `design/notification-center-alerting-plan.md`、`crates/tikeo-server/src/notification.rs`、`crates/tikeo-server/src/http/routes/notifications.rs`、`crates/tikeo-storage/src/repository/notification.rs` 和 `web/src/pages/NotificationCenterPage.tsx`。如果这些证据和文档冲突，应该修文档或代码，并加测试，而不是用含糊话术掩盖。

## 下一步

新评估者先读 [安装](./getting-started/installation) 和 [快速开始](./getting-started/quickstart)。SDK 接入者先读 [配置参考](./reference/configuration)，再读对应语言 SDK。Kubernetes 运维读 [Kubernetes 与 Helm](./deployment/kubernetes)、[Server 高可用与 Raft FSOD 集群](./deployment/server-ha) 和 [Kubernetes 控制器 runbook](./deployment/kubernetes-controller-runbook)。通知 operator 应阅读 [通知用户指南](./user-guide/notifications)、[告警用户指南](./user-guide/alerts) 和 [通知中心参考](./reference/notification-center)，保持出站投递与 incident 语义分离。排障时使用 [故障排查](./reference/troubleshooting)、smoke 报告目录和源码派生参考。

## 前置条件

执行本页命令前，请先满足页面列出的安装、认证和权限要求。本地示例默认 Server 使用 `config/dev.toml`，客户端访问 `127.0.0.1`，令牌保存在 shell 变量中，不写入文件或截图。

## 验收

完成本页步骤后，用对应 API、UI、构建、smoke 或部署检查验证结果。有效验收至少包含执行的命令、检查的路由或文件，以及观察到的状态或产物。

## 故障排查

步骤失败时，先保留完整命令、响应状态和 Server 日志时间窗口，再检查认证、namespace/app scope、Worker 匹配、存储 readiness 和代理行为，不要直接修改生产配置。

## 生产检查清单

- [ ] 密钥通过环境变量或平台 Secret 引用管理，不写入示例。
- [ ] 已把本地 `127.0.0.1` 命令替换成真实域名、TLS 和认证方式。
- [ ] 已记录变更面的回滚和证据采集方式。
- [ ] 运维人员可以在没有隐藏 shell 历史或隐式状态的情况下复现验收。
