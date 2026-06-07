<p align="center">
  <img src="web/src/assets/tikeo-logo.svg" alt="Tikeo logo" width="148" height="148" />
</p>

<h1 align="center">Tikeo</h1>
<p align="center"><strong>面向已经超越传统任务调度器阶段团队的开源任务编排平台。</strong></p>

<p align="center">
  <a href="README.md">🇺🇸 English</a> ·
  <a href="deploy/compose/README.md">🐳 Docker Compose</a> ·
  <a href="sdks/README.md">🧩 SDKs</a> ·
  <a href="examples/README.md">🚀 Examples</a> ·
  <a href="deploy/terraform/README.md">🌍 Terraform</a> ·
  <a href="deploy/k8s/operator/README.md">☸️ Operator</a>
</p>

<p align="center">
  <a href="https://github.com/yhyzgn/tikeo/actions/workflows/ci.yml"><img alt="CI build" src="https://img.shields.io/github/actions/workflow/status/yhyzgn/tikeo/ci.yml?branch=main&style=flat-square&label=CI%20build" /></a>
  <a href="https://github.com/yhyzgn/tikeo/releases"><img alt="Latest release" src="https://img.shields.io/github/v/release/yhyzgn/tikeo?include_prereleases&style=flat-square&label=release" /></a>
  <img alt="Current version" src="https://img.shields.io/badge/current-v0.1.0-0f172a?style=flat-square" />
  <img alt="Coverage" src="https://img.shields.io/badge/coverage-report%20pending-f97316?style=flat-square" />
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/yhyzgn/tikeo?style=flat-square" /></a>
</p>

<p align="center">
  <img alt="Java SDK" src="https://img.shields.io/badge/Java%20SDK-net.tikeo%3Atikeo%400.1.0--SNAPSHOT-b07219?style=flat-square&logo=openjdk" />
  <img alt="Rust SDK" src="https://img.shields.io/badge/Rust%20SDK-tikeo%400.1.0-ce422b?style=flat-square&logo=rust" />
  <img alt="Go SDK" src="https://img.shields.io/badge/Go%20SDK-github.com%2Fyhyzgn%2Ftikeo%2Fsdks%2Fgo%2Ftikeo-00add8?style=flat-square&logo=go" />
  <img alt="Python SDK" src="https://img.shields.io/badge/Python%20SDK-tikeo%400.1.0-3776ab?style=flat-square&logo=python" />
  <img alt="Node.js SDK" src="https://img.shields.io/badge/Node.js%20SDK-%40yhyzgn%2Ftikeo%400.1.0-339933?style=flat-square&logo=nodedotjs" />
</p>

<p align="center">
  <img alt="Server image" src="https://img.shields.io/badge/Docker-yhyzgn%2Ftikeo--server-2563eb?style=flat-square&logo=docker" />
  <img alt="Web image" src="https://img.shields.io/badge/Docker-yhyzgn%2Ftikeo--web-2563eb?style=flat-square&logo=docker" />
  <img alt="Sandbox" src="https://img.shields.io/badge/sandbox-SRT%20%7C%20Deno%20%7C%20WASM%20%7C%20V8-7c3aed?style=flat-square" />
  <img alt="Databases" src="https://img.shields.io/badge/storage-SQLite%20%7C%20Postgres%20%7C%20MySQL-0891b2?style=flat-square" />
  <img alt="Observability" src="https://img.shields.io/badge/observability-OpenTelemetry-0f766e?style=flat-square" />
  <img alt="IaC" src="https://img.shields.io/badge/IaC-Helm%20%7C%20Terraform%20%7C%20CRD-f97316?style=flat-square" />
</p>

---

## 不要再选择“只会调度”的调度器

XXL-Job 和 PowerJob 推动了实用型分布式任务执行的普及。Tikeo 面向下一个阶段：平台团队需要的不只是调度器，而是一个调度器、工作流引擎、Worker 集群控制面、脚本治理层和可发布 SDK 共同组成的统一开源系统。

Tikeo 的目标，是在有人提出下面这个问题时，成为默认答案：

> “云原生任务调度、工作流编排、脚本任务、Worker 治理，以及可观测执行证据，我们应该选什么？”

## 10 秒速览：为什么值得关注

| 信号 | 为什么重要 |
| --- | --- |
| **5 条生产级 SDK 轨道** | **Java · Rust · Go · Python · Node.js** Worker 遵循同一份契约，而不是只能围绕 Java-first 执行器模型建设。 |
| **出站 Worker Tunnel** | Worker 主动连接服务端；生产业务服务不需要暴露入站任务执行端口。 |
| **结构化能力路由** | 调度匹配类型化的 **SDK Processor**、**插件 Processor** 和 **脚本 Runner**，不再依赖魔法字符串解析。 |
| **沙箱优先的脚本任务** | `auto` 对原生脚本选择 **SRT**，对 JS/TS 选择 **Deno**，同时也支持显式使用 **WASM/V8/container** 路径。 |
| **工作流 + 拓扑 UX** | 可视化工作流画布、依赖拓扑、影响分析、回放数据，以及广播任务的按 Worker 结果。 |
| **运维级执行证据** | **重试**、**Misfire 策略**、**任务日志**、**审计日志**、**OpenTelemetry**、指标与文件日志一起回答“到底发生了什么”。 |
| **云原生发布面** | Docker、Compose、Helm、Kubernetes CRD/operator、Terraform provider、GitOps diff 和跨平台 release assets。 |

<p align="center">
  <strong>关键词：</strong>
  <kbd>Rust control plane</kbd>
  <kbd>Worker Tunnel</kbd>
  <kbd>Structured Capabilities</kbd>
  <kbd>Script Sandbox</kbd>
  <kbd>Workflow Canvas</kbd>
  <kbd>RBAC</kbd>
  <kbd>OpenTelemetry</kbd>
  <kbd>Terraform</kbd>
  <kbd>K8s Operator</kbd>
</p>

## 产品承诺

| 承诺 | 在实践中意味着什么 |
| --- | --- |
| 🧠 **一个编排大脑** | **Cron**、**fixed-rate**、**API 触发**、**广播**、**工作流**、**脚本**、**插件** 和 **SDK** 任务共享同一个受治理的实例模型。 |
| 🔌 **无需暴露执行器端口** | Worker 通过 **出站 gRPC tunnel** 主动连接；业务服务保持在正常网络边界之后。 |
| 🧱 **类型化调度，不靠传说约定** | 路由使用结构化 **SDK processors**、**plugin processor types**、**script languages**、**sandbox backends**、标签和选举字段。 |
| 🛡️ **脚本就是受治理的工作负载** | 不可变版本、摘要校验、审批元数据、策略限制、任务级日志和沙箱自动选择都是一等能力。 |
| 🧩 **SDK 天生对齐** | **Java/Rust/Go/Python/Node.js** 在 Worker 注册、任务日志、重试、Management API、沙箱行为和诊断上保持一致。 |
| 📈 **证据优先的运维体验** | 实例结果、重试日志、广播 Worker 分组、终端风格日志、审计轨迹、OTel trace、指标和 GitOps diff 都是内建能力。 |

## 创新地图

| 创新点 | Tikeo 优势 | 它消除了哪些传统痛点 |
| --- | --- | --- |
| **Worker Tunnel** | Worker 通过带 lease/fencing 元数据的出站 tunnel 拉取任务。 | 入站执行器暴露和脆弱的回调假设。 |
| **Capability Graph** | Worker 能力是类型化图：SDK processors、plugins、scripts、tags、election domains。 | 模糊字符串约定，以及“为什么这个 Worker 收到这个任务”的排障困难。 |
| **Sandbox Auto Strategy** | `auto` 选择最安全且实用的运行时路径：原生脚本走 SRT，JS/TS 走 Deno，适当时走 Wasmtime/WASM。 | 把脚本当普通 shell command 执行，隔离边界不清。 |
| **Execution Evidence Model** | 每次 attempt、retry、worker result、broadcast child 和 task log 都可检查。 | 只能看状态、无法解释失败原因的控制台。 |
| **Open Platform Surface** | SDKs、Docker、Helm、Terraform、CRD/operator、GitOps diff、OpenAPI、OTel。 | 调度器因缺少集成面而阻碍落地。 |

## 为什么评估者应该优先把 Tikeo 放进候选名单

### 1. 它覆盖的是更完整的真实平台问题

传统调度器往往停留在“在执行器上触发一个任务”。Tikeo 覆盖生产团队迟早会需要的外围能力：RBAC、owner 初始化、app 作用域 API Key、租户范围、插件处理器、脚本沙箱、拓扑、可回放日志、GitOps drift review、Terraform、Kubernetes CRD、Helm、Docker 镜像和 SDK 发布。

### 2. 它避开了约定式路由的隐性成本

依赖魔法字符串的调度器最终会变得难以运维。Tikeo 使用结构化能力声明进行路由。Worker 明确声明自己能运行什么，服务端显式匹配类型化 SDK processors、plugin processor types，以及脚本语言/后端。

### 3. 它把脚本执行当成安全产品，而不是一个勾选项

Tikeo 的脚本模型默认假设脚本强大且有风险。平台区分脚本类型与沙箱后端，支持 `auto` 沙箱选择，并能解析 SRT/Deno/WASM 相关路径；除非明确指定，否则不会默认走重量级 Docker/Podman。

### 4. 它面向开源采用和中央包仓库发布而建设

仓库包含独立 SDK 包、示例、Compose 栈、Helm/K8s/Terraform 资产、发布流水线和文档入口。它是给真实团队消费的产品，而不是只能研究的 demo。

## 决策摘要

| 当你需要……就选择 Tikeo | 为什么这是决定性因素 |
| --- | --- |
| **一个平台，而不是一个定时器** | Jobs、workflows、workers、scripts、plugins、RBAC、audit 和 IaC 被统一设计。 |
| **多语言 Worker 采用** | 团队可以继续用 Java、Rust、Go、Python 或 Node.js 写业务代码，同时保持平台一致性。 |
| **安全意识更强的脚本执行** | 脚本治理和沙箱选择是模型的一部分，而不是事后补丁。 |
| **云原生运维模型** | Kubernetes、Terraform、Docker、OTel 和 release assets 是项目的一等入口。 |
| **清晰的失败取证** | 任务日志、重试日志、Worker attempts、审计轨迹和拓扑让失败可复盘。 |

## Tikeo vs. XXL-Job vs. PowerJob

这不是“功能数量炫耀”。这是任务调度器和编排平台之间的差异。

| 评估维度 | Tikeo | XXL-Job | PowerJob |
| --- | --- | --- | --- |
| **平台定位** | ✅ **完整编排平台**：jobs、workflows、workers、scripts、plugins、RBAC、observability、IaC。 | 成熟的 Java 任务调度器。 | 成熟的 Java 分布式任务平台。 |
| **Worker 连接模型** | ✅ 带 lease/fencing 和结构化注册的 **出站 Worker Tunnel**。 | 执行器注册/回调风格。 | Worker server 模型。 |
| **路由契约** | ✅ **类型化 SDK/plugin/script capabilities**，不解析约定字符串。 | 偏名称/字符串。 | 偏名称/tag。 |
| **语言生态** | ✅ **Java · Rust · Go · Python · Node.js** SDK 对齐。 | 主要是 Java 生态。 | 主要是 Java 生态。 |
| **脚本执行** | ✅ **受治理版本 + 摘要校验 + SRT/Deno/WASM/V8/container** 策略。 | 有脚本执行，但不是完整的沙箱治理产品。 | 偏 Processor；沙箱治理不是核心。 |
| **工作流 UX** | ✅ **工作流画布 + 拓扑 + 影响分析 + 可回放执行数据。** | 以调度为中心的基础视图。 | 支持工作流，但较少聚焦类型化沙箱 + SDK 对齐。 |
| **安全模型** | ✅ **Owner 初始化、RBAC 矩阵、不透明 session、API keys、租户范围、审计轨迹。** | 管理员/用户模型。 | 管理员/用户模型。 |
| **可观测性** | ✅ **OpenTelemetry、metrics、task logs、file logs、audit logs、worker grouping。** | 传统运维/日志。 | 传统运维/日志。 |
| **云原生资产** | ✅ **Docker、Compose、Helm、K8s CRD/operator、Terraform provider、GitOps diff。** | 可部署，但不是 GitOps/IaC-first。 | 可部署，但不是 GitOps/IaC-first。 |
| **最佳适用场景** | 建设内部编排平台的团队，而不只是替换 cron。 | 希望使用熟悉调度器的 Java 团队。 | 希望使用分布式任务执行的 Java 团队。 |

**简短结论：** 当你想要现代编排控制面时选择 Tikeo；只有当你明确只想要更窄的 Java-first 调度器时，才选择传统调度器。

### 评估清单

如果你的调度器候选清单包含下面这些要求，Tikeo 应该被放到最前面：

- [x] **多语言 Worker**，同时不丢失统一平台契约。
- [x] **工作流 + 拓扑可视化**，而不是只有任务列表式运维。
- [x] **脚本沙箱治理**，具备显式后端选择和默认轻量 auto 模式。
- [x] **RBAC + API-Key + audit**，用于真实后台管理。
- [x] **OpenTelemetry + metrics + durable logs**，用于生产排障。
- [x] **Helm + Terraform + K8s CRD/operator**，面向平台工程团队。

## 架构

<p align="center">
  <img src="docs/assets/tikeo-architecture.svg" alt="Tikeo architecture diagram" width="100%" />
</p>

服务端负责调度、持久化、治理、RBAC、工作流和派发决策。Worker 负责执行，并声明自己能安全运行什么。脚本以不可变版本派发，只会由暴露兼容沙箱 runner 的 Worker 执行。

### 核心流程

| 流程 | 发生了什么 |
| --- | --- |
| **任务调度** | Cron/fixed/API triggers 创建实例，应用 retry/misfire 策略，并入队派发工作。 |
| **Worker 注册** | Worker 拨入 tunnel，发送结构化能力，接收权威 `worker_id`，并续约 lease。 |
| **派发** | 服务端在分配任务前匹配 namespace/app、worker state、master election 和类型化能力。 |
| **执行证据** | Worker 发送任务级日志和结果 payload；广播模式存储按 Worker 的 attempts 和 outcomes。 |
| **治理** | RBAC、API keys、tenant scopes、script approvals、audit logs 和 GitOps diff 让变更可复盘。 |

## 能证明产品价值的快速开始

### 1. 启动控制面

```bash
./scripts/dev.sh
```

这会启动 Rust server 和 React web console，把日志流式输出到终端，同时也把本地日志写入 `.dev/`。

打开 <http://127.0.0.1:5173>。全新数据库会进入首次 owner 设置页面。owner 创建完成后注册入口关闭，用户和角色在控制台内部管理。

### 2. 写入真实评估数据

```bash
./scripts/dev-seed.sh
```

种子数据会提供 namespaces、apps、示例任务、脚本、工作流、审计记录和实例日志，让你可以立即评估控制台，而不是面对空产品。

### 3. 用你偏好的语言启动 Worker

```bash
# Rust
(cd examples/rust/worker-demo && cargo run)

# Go
(cd examples/go/worker-demo && go run .)

# Python
(cd examples/python/worker-demo && python -m pip install -e ../../../sdks/python/tikeo -e . && python -m tikeo_python_worker_demo)

# Node.js / Bun
(cd examples/nodejs/worker-demo && bun install && bun start)

# Java / Spring Boot 4
(cd examples/java/spring-boot4-worker-demo && ./scripts/run-demo-worker.sh)
```

### 4. 触发并检查

在 Web 控制台中：

1. 打开 **Workers**，确认 Worker 已出现并显示结构化能力。
2. 打开 **Jobs**，触发一个种子 SDK/script/plugin 任务。
3. 打开 **Instances**，检查状态、重试 attempts、按 Worker 的广播结果和终端风格日志。
4. 打开 **Topology** 或 **Workflows**，检查依赖关系和可视化编排。

这条路径会验证完整价值主张：**control plane**、**worker tunnel**、**SDK execution**、**capability matching**、**task logs**、**retry/result evidence** 和 **visual operations**。

快速开始后的预期证据点：

| 证据点 | 在哪里查看 |
| --- | --- |
| **Worker 已连接** | Workers 页面显示已注册 Worker 和结构化能力。 |
| **派发是结构化的** | 任务触发时按 namespace/app 和类型化 processor/script/plugin capability 选择 Worker。 |
| **执行可解释** | Instances 页面显示状态、重试进度、worker id、结果和终端日志。 |
| **工作流可见** | Workflow 和 topology 页面显示依赖，而不是把编排隐藏在代码里。 |

## 你可以用 Tikeo 构建什么

这些不是需要你再拼装的独立产品。它们都是 Tikeo 的运行模式。

| 场景 | 高价值关键词 | Tikeo 如何帮助 |
| --- | --- | --- |
| **内部平台调度器** | `Worker Tunnel` · `RBAC` · `API-Key` | 让每个服务团队以受治理方式注册 processors 和触发任务，同时不开放入站端口。 |
| **数据与对账任务** | `Retry` · `Misfire` · `Task Logs` | 通过 retries、logs、app scopes 和多语言 SDK 运行周期性或 API 触发任务。 |
| **脚本运维中心** | `SRT` · `Deno` · `WASM` · `Digest` | 审批脚本、发布不可变版本、在声明沙箱中运行，并把输出绑定到实例。 |
| **工作流自动化** | `Canvas` · `Topology` · `Replay` | 将任务组合成可视化工作流，并在修改依赖前检查拓扑/影响。 |
| **Kubernetes 平台集成** | `Helm` · `CRD` · `Terraform` | 使用 Helm、CRDs、operator status、Terraform diff 和 Docker images，无需重写调度器。 |
| **可审计运维** | `Audit` · `OTel` · `Worker Results` | 追踪谁改了什么、哪个 Worker 运行了什么、为什么派发失败，以及每次重试发生了什么。 |

## 运维真正需要的配置

配置文件位于 `config/`，并可通过 `TIKEO__...` 环境变量覆盖。

```toml
[storage]
database_url = "postgres://tikeo:tikeo@postgres:5432/tikeo"

[observability.logging]
level = "info"
log_dir = "./logs"

[observability.tracing]
enabled = true
otlp_endpoint = "http://otel-collector:4318/v1/traces"
```

存储支持：

| Backend | 推荐用途 |
| --- | --- |
| SQLite | 本地开发、demo、单节点 smoke validation。 |
| PostgreSQL | 生产和共享环境。 |
| MySQL | 生产环境中 MySQL 是平台标准时。 |
| CockroachDB-compatible PostgreSQL wire | 使用 PostgreSQL 协议兼容能力的分布式 SQL 环境。 |

## 行为一致的 SDK

| Language | Package | 适合什么 | 日志契约 |
| --- | --- | --- | --- |
| Java | `net.tikeo:tikeo`, Spring Boot starters | 企业 Spring Worker 和管理自动化。 | SLF4J diagnostics；任务日志通过 `TaskContext`。 |
| Rust | `tikeo` | 原生 Worker、高性能运行时、具备沙箱能力的服务。 | `SdkLogConfig`，console + 可选 `tikeo-sdk.log`。 |
| Go | Go module | 平台服务、operators、云原生 Worker。 | `Logger` bridge，console + 可选 `tikeo-sdk.log`。 |
| Python | `tikeo` | 数据任务、自动化、脚本友好 Worker。 | stdlib `logging`，console + 可选 `tikeo-sdk.log`。 |
| Node.js | `@yhyzgn/tikeo` | JS/TS Worker 和 Web 平台自动化。 | `configureSdkLogging`，console + 可选 `tikeo-sdk.log`。 |

所有 SDK 遵循同一条规则：SDK diagnostics 描述 Worker/runtime 生命周期；task logs 描述某个具体任务实例。这个分离能防止无关进程噪音污染执行日志。

## 部署路径

| 路径 | 适用时机 |
| --- | --- |
| `docker-compose.yml` | 想用 SQLite 最快完成本地产品评估。 |
| `docker-compose.postgres.yml` / `docker-compose.mysql.yml` | 想验证真实数据库可移植性。 |
| `deploy/systemd/` | 在 VM 或裸机主机上运行 Tikeo。 |
| `deploy/helm/tikeo/` | 将控制面部署到 Kubernetes。 |
| `deploy/k8s/operator/` | 希望使用 CRD-based GitOps drift review。 |
| `deploy/terraform/provider/` | 希望在 Terraform 工作流中导出/对比 manifest。 |

## 可观测性与故障排查

Tikeo 的设计目标，是让运维人员能回答真正重要的问题：

- **为什么这个实例派发了或没有派发？** 查看实例日志和 capability/governance 消息。
- **哪个 Worker 执行了任务？** 检查实例结果和广播 Worker 分组。
- **脚本输出了什么？** 阅读任务级终端日志，而不是泛化进程日志。
- **失败前发生了什么变更？** 使用审计日志、GitOps diff 和 job/workflow versions。
- **延迟来自哪里？** 使用 OpenTelemetry、metrics 和 SDK/server diagnostics。

## 仓库地图

```text
crates/            Rust server crates, scheduling, storage, worker tunnel, HTTP API
web/               React + Ant Design management console
sdks/              Java, Rust, Go, Python, and Node.js SDKs
examples/          Runnable worker demos per language
deploy/            Compose, Helm, K8s operator, Terraform provider, systemd, smoke tests
docs/              Operations, reports, localized docs, and README assets
design/            Architecture and roadmap records
scripts/           Development, seeding, release, and verification helpers
```

## 验证

```bash
cargo test --workspace
(cd web && bun run typecheck && bun run build)
(cd sdks/java && ./gradlew test --no-daemon)
(cd sdks/rust/tikeo && cargo test --all-features)
(cd sdks/go/tikeo && go test ./...)
(cd sdks/python/tikeo && python -m pytest)
(cd sdks/nodejs/tikeo && bun test && bun run build)
```

## License

Apache-2.0。大胆构建，谨慎运维，并让执行证据保持精确。
