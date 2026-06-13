---
title: Rust Worker SDK
description: Rust SDK 与 Worker demo 的 operator-grade 验收入口。
---

# Rust Worker SDK

Rust SDK 位于 `sdks/rust/tikeo`，可运行 Worker demo 位于 `examples/rust/worker-demo`。本文只记录源码中能核对的事实：配置来自 `src/config.rs`，Management helper 来自 `src/management.rs`，demo 行为来自 `examples/rust/worker-demo/src/main.rs`。Worker 是 **outbound-only**：进程主动连接 Server 的 Worker Tunnel、注册能力、接收 `DispatchTask`，再通过同一 tunnel 回传日志和结果；不要把业务 Worker 写成 inbound HTTP Service，也不要要求 Server 主动回调 Worker。

## 依赖坐标

发布 crate 名称是 `tikeo`，当前源码 manifest 是 `sdks/rust/tikeo/Cargo.toml`：`name = "tikeo"`、`version = "0.2.0"`、`edition = "2024"`、`rust-version = "1.95"`。安装时使用不带 `v` 的版本号：

```bash
cargo add tikeo@${TIKEO_VERSION}
```

或在 `Cargo.toml` 中声明：

```toml
[dependencies]
tikeo = "${TIKEO_VERSION}"
```

本仓库内 demo 使用路径依赖，实际集成时应切换到发布版本或内部 registry 固定版本。Rust SDK 导出 `WorkerConfig`、`WorkerClient`、`TaskContext`、`TaskOutcome`、`TaskProcessor`、`ManagementClient`、`ManagementCreateJobRequest`、`ManagementTriggerJobRequest`、`ManagementBroadcastSelectorRequest` 等符号。

## WorkerConfig 默认值

`WorkerConfig::local(endpoint, client_instance_id)` 是源码里的最小开发配置。它不会猜测生产 scope，只填入可本地启动的默认值：`endpoint` 来自参数，`client_instance_id` 来自参数，`app="default"`，`namespace="default"`，`cluster="local"`，`region="local"`，`capabilities=[]`，`structured_capabilities=WorkerCapabilities::default()`，`labels={}`。注册消息会把 `WorkerClusterElection` 写为 `enabled=true`、`domain=""`、`priority=100`。

`examples/rust/worker-demo/src/main.rs` 在 demo 层覆盖这些默认值：`TIKEO_WORKER_ENDPOINT` 默认 `http://127.0.0.1:9998`，`TIKEO_WORKER_CLIENT_INSTANCE_ID` 默认 `rust-worker-demo-local`，`TIKEO_WORKER_NAMESPACE` 默认 `dev-alpha`，`TIKEO_WORKER_APP` 默认 `orders`，`TIKEO_WORKER_CLUSTER` 和 `TIKEO_WORKER_REGION` 默认 `local`，`worker_pool` 默认 `rust-blue`。demo 会添加 tag `rust`、`manual-demo`，并默认广告 `demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail,demo.exception` 这些 SDK processor。插件 SQL 默认启用路径来自 `enabled_by_default("TIKEO_ENABLE_PLUGIN_SQL")`，会添加 plugin processor `type=sql`、`processorName=billing.sql-sync`，并标记 label `plugin_sql=enabled`。

## 最小 Worker

最小 Worker 只需要创建 config、声明真实 processor、构造 client，并在 outbound tunnel 上处理任务。下面代码保留了源码符号，但省略了 demo 中的多语言脚本 runner；生产时只有当 runner 真的可用时才调用 `add_script_runner`。

```rust
use async_trait::async_trait;
use tikeo::{TaskContext, TaskOutcome, TaskProcessor, WorkerClient, WorkerConfig, WorkerSdkError};

struct Echo;

#[async_trait]
impl TaskProcessor for Echo {
    async fn process(&self, context: TaskContext) -> Result<TaskOutcome, WorkerSdkError> {
        context.log_info("rust echo started");
        Ok(TaskOutcome::Success("rust demo echo processed".to_owned()))
    }
}

#[tokio::main]
async fn main() -> Result<(), WorkerSdkError> {
    let endpoint = std::env::var("TIKEO_WORKER_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:9998".to_owned());
    let mut config = WorkerConfig::local(endpoint, "rust-worker-demo-local");
    config.namespace = "dev-alpha".to_owned();
    config.app = "orders".to_owned();
    config.add_tag("rust");
    config.add_sdk_processor("demo.echo");

    let client = WorkerClient::new(config);
    let mut session = client.connect().await?;
    let _outcome = session.process_next(&Echo).await?;
    session.close().await?;
    Ok(())
}
```

操作约束：`add_sdk_processor`、`add_script_runner`、`add_plugin_processor` 只是能力广告，不会自动让宿主具备能力。缺少 SRT、Deno、container、WASM 或插件运行时的时候，应 fail closed，不要把不可执行能力发布给调度器。任务日志必须通过 `TaskContext` 发送，才能绑定到 job instance 与 assignment token；SDK 诊断日志用于连接、注册、心跳、sandbox 解析，不等于业务任务日志。

## Management API 与管理客户端凭证

Rust management client 的事实来源是 `sdks/rust/tikeo/src/management.rs`。`ManagementClient::new(endpoint, api_key, namespace, app)` 会 trim endpoint 尾部斜杠，把空 namespace/app 变成 `default`，HTTP timeout 为 30 秒，并在每次请求发送 `x-tikeo-api-key` 与 `accept: application/json`。凭证应来自 Secret store 或环境变量 `TIKEO_MANAGEMENT_API_KEY`；不要把浏览器 session、OIDC cookie 或人类 bearer token 传给 Worker。源码 helper 名称如下：

- `ManagementCreateJobRequest::api(name, processor_name)`：创建 `scheduleType=api` 的 SDK processor job，默认 `enabled=true`，`retryPolicy` 为 enabled、3 次、5 秒初始延迟、2 倍退避、60 秒上限。
- `ManagementCreateJobRequest::plugin_api(name, processor_type, processor_name)`：创建 plugin job，写入 `processorType` 和 `processorName`。
- `ManagementCreateJobRequest::script_api(name, script_id)`：创建 script job，写入 `scriptId`。
- `ManagementTriggerJobRequest::api()`：发送 `triggerType=api` 和 `executionMode=single`。
- `ManagementTriggerJobRequest::broadcast_api(selector)`：发送 `triggerType=api`、`executionMode=broadcast` 与可选 `broadcastSelector`。

```rust
use tikeo::{
    ManagementBroadcastSelectorRequest,
    ManagementClient,
    ManagementCreateJobRequest,
    ManagementTriggerJobRequest,
};

let management = ManagementClient::new(
    std::env::var("TIKEO_MANAGEMENT_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:9090".to_owned()),
    std::env::var("TIKEO_MANAGEMENT_API_KEY")?,
    "dev-alpha",
    "orders",
);
let created = management
    .create_job(ManagementCreateJobRequest::api("rust-echo-api", "demo.echo"))
    .await?;
let instance = management
    .trigger_job(&created.id, ManagementTriggerJobRequest::api())
    .await?;
assert_eq!(instance.trigger_type, "api");
assert_eq!(instance.execution_mode, "single");

let selector = ManagementBroadcastSelectorRequest {
    tags: Some(vec!["manual-demo".to_owned()]),
    region: Some("us-east-1".to_owned()),
    cluster: None,
    labels: Some(std::collections::HashMap::from([("worker_pool".to_owned(), "rust-blue".to_owned())])),
};
let _ = management
    .trigger_job(&created.id, ManagementTriggerJobRequest::broadcast_api(Some(selector)))
    .await?;
```

参考锚点：[`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)、[`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)、[`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)、[`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)、[`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)。

## Demo 行为与能力边界

Rust demo 默认配置 SDK processor、插件 SQL processor，以及可选脚本 runner。脚本 runner 的 sandbox 后端由 `TIKEO_WORKER_SCRIPT_SANDBOX` 控制；auto 模式会按语言选择 SRT、Deno、container 或 unavailable adapter。`TIKEO_SANDBOX_AUTO_INSTALL=0/false/no/off` 会关闭工具自动安装。`TIKEO_ENABLE_UNAVAILABLE_SCRIPT_ADAPTERS` 只有在显式打开时才注册不可用 adapter；正常验收应要求不可用运行时不被广告。`TIKEO_WORKER_DRY_RUN=1` 或关闭连接时，demo 只验证本地注册与 heartbeat，不会连 live tunnel。

## 失败与异常 demo

所有语言 demo 都区分业务失败和运行时异常。`demo.fail` 返回正常的 failed `TaskOutcome`，用于验证业务规则失败；`demo.exception` 会 throw、panic、raise 或返回 processor error，用于验证 SDK 能把真实异常栈作为任务日志透传，同时仍把实例结果标记为失败。验收时两个 processor 都要触发：前者证明业务失败语义，后者证明异常堆栈能穿过 Worker Tunnel 并出现在 Notification Center 的执行透传页面。

## 运维依据与排错边界

核对 Rust 集成时，先看 `sdks/rust/tikeo/src/config.rs` 中注册消息如何序列化，再看 `src/session.rs` 中 heartbeat、`process_next_with_script_runners`、assignment token 日志和 `TaskResult` 的发送顺序。`src/task.rs` 说明 `TaskOutcome::Succeeded`、`TaskOutcome::Success` 与 `TaskOutcome::Failed` 才是 Worker 结果边界；普通 stdout 只能辅助排错，不能替代实例日志。`examples/rust/worker-demo/src/main.rs` 是 operator demo，不是框架模板：它把 namespace、app、cluster、region、labels、plugin SQL 和脚本 runner 都放到环境变量后面，方便现场逐项开关。遇到任务未派发时，不要先修改业务 processor；先核对 Worker 是否注册到了同一 namespace/app，`structured_capabilities.sdk_processors` 是否包含目标 processor，label 和 `broadcastSelector` 是否收敛到预期池，脚本 runner 是否因为工具缺失而没有广告。

## 生产上线检查

上线前把 Rust Worker 当作独立运行单元管理。配置应由部署系统注入，而不是写死在二进制里；`client_instance_id` 可以稳定关联重连，但 Server 分配的 `worker_id` 才是权威运行身份。多副本部署时，用 namespace、app、cluster、region 和 `worker_pool` label 描述调度边界，不要依赖临时主机名。脚本能力、插件能力和 SDK processor 都应经过发布评审：新增一个 processor 名称等同于扩大调度入口，新增一个 script runner 等同于扩大宿主执行面。Management API key 应只授予对应 namespace/app 的控制面动作，轮换后重启 Worker 或刷新注入配置；任何日志、panic、debug dump 都不得输出密钥值。

## 现场验收 runbook

1. 构建 SDK：`cargo fmt --manifest-path sdks/rust/tikeo/Cargo.toml -- --check`，再执行 `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` 和 `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features`。
2. 验证 demo dry-run：`TIKEO_WORKER_DRY_RUN=1 cargo run --manifest-path examples/rust/worker-demo/Cargo.toml`，确认输出包含 registration、`dry_run_heartbeat_sequence`，且没有尝试监听业务 HTTP 端口。
3. 启动 Server 后跑 live：设置 `TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998`、namespace/app/cluster/region、`TIKEO_WORKER_POOL=rust-blue`，启动 demo，确认 Web 控制台能看到 outbound Worker session。
4. 用 management helper 创建 `demo.echo` API job，确认请求携带 `x-tikeo-api-key`，密钥来自 `TIKEO_MANAGEMENT_API_KEY`，触发响应为 `triggerType=api` 与 `executionMode=single`。
5. 只在需要 fan-out 时测试 `broadcastSelector`；选择 tag `manual-demo` 和 label `worker_pool=rust-blue`，确认只匹配预期 Worker。
6. 查看实例日志和结果，确认 processor 日志通过 `TaskContext` 绑定到 instance；断开 Worker 后确认 Server stale worker fencing、重连、注销路径符合预期。

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
