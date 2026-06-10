---
title: Rust Worker SDK
description: Rust SDK 与 Worker demo 的验证入口。
---

# Rust Worker SDK

Rust SDK 位于 `sdks/rust/tikeo`，可运行 Worker demo 位于 `examples/rust/worker-demo`。它是 Tikeo 最接近 Server 内核协议的原生 SDK，也是验证 Worker Tunnel、能力广告、日志与结果回传的推荐入口。

## 运行时要求

Rust SDK 当前按 README 与 CI 声明的 Rust 1.95+ 基线维护。若未来调整 Rust toolchain，必须同步 SDK 文档、demo manifest、CI setup 和徽章，避免宣传与实际构建漂移。

## 从 crates.io 安装

将 `${TIKEO_VERSION}` 替换为 README 顶部 `Rust SDK` 徽标显示的版本号。Rust 使用不带 `v` 的版本字符串。

```bash
cargo add tikeo@${TIKEO_VERSION}
```

```toml
[dependencies]
tikeo = "${TIKEO_VERSION}"
```

## 验证 SDK

```bash
cargo fmt --manifest-path sdks/rust/tikeo/Cargo.toml -- --check
cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
```

## 运行 demo

```bash
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

live mode 下，demo 应从本地配置连接 Worker Tunnel endpoint。


## Management API 创建并触发任务

Rust management client 的事实来源是 `sdks/rust/tikeo/src/management.rs`。它只面向 app 级服务凭据：SDK 会发送 `x-tikeo-api-key`，通常从 `TIKEO_MANAGEMENT_API_KEY` 注入；不要把浏览器 session、OIDC cookie 或用户 bearer token 传给 Worker。创建 API 任务时使用 `scheduleType=api`；默认触发 helper 会发送 `triggerType=api` 和 `executionMode=single`。

```rust
use tikeo::{
    ManagementBroadcastSelectorRequest,
    ManagementClient,
    ManagementCreateJobRequest,
    ManagementTriggerJobRequest,
};

let endpoint = std::env::var("TIKEO_MANAGEMENT_ENDPOINT")
    .unwrap_or_else(|_| "http://127.0.0.1:9090".to_owned());
let api_key = std::env::var("TIKEO_MANAGEMENT_API_KEY")?;
let management = ManagementClient::new(endpoint, api_key, "dev-alpha", "orders");

let created = management
    .create_job(ManagementCreateJobRequest::api("rust-echo-api", "demo.echo"))
    .await?;
let instance = management
    .trigger_job(&created.id, ManagementTriggerJobRequest::api())
    .await?;

assert_eq!(instance.trigger_type, "api");
assert_eq!(instance.execution_mode, "single");
```

广播不是默认行为。只有需要一次 API 触发扇出到多个匹配 Worker 时，才使用显式 selector helper；它会序列化 `broadcastSelector` 并设置 `executionMode=broadcast`。

```rust
let broadcast = ManagementTriggerJobRequest::broadcast_api(Some(
    ManagementBroadcastSelectorRequest {
        tags: Some(vec!["manual-demo".to_owned()]),
        region: Some("us-east-1".to_owned()),
        cluster: None,
        labels: Some(std::collections::HashMap::from([(
            "worker_pool".to_owned(),
            "rust-blue".to_owned(),
        )])),
    },
));
let _instance = management.trigger_job(&created.id, broadcast).await?;
```


## Source-backed 参考链接

SDK helper 文档必须锚定到从源码整理出的 API 与协议参考：

- 创建 helper 端点：[`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- 触发 helper 端点：[`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- 实例轮询端点：[`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- 实例日志端点：[`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker 派发消息：[`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## Worker 心智模型

Rust Worker 负责三件事：连接 Server tunnel，只广告真实可执行能力，并带着 Server 下发的 assignment token 回传日志和结果。这样调度、审计、stale worker fencing 才能保持一致。

## 能力广告纪律

如果 Worker 不能执行某个 processor、script backend 或 plugin capability，就不要广告它。不可用 runtime 应 fail closed，而不是伪装成可用能力。

## 生产建议

生产环境应把 Worker 独立打包，而不是塞进 Server 镜像。Worker identity 应通过 namespace、app、worker pool、labels 和 structured capabilities 表达，不要依赖临时命名约定。

## 适合场景

Rust Worker 适合对延迟、资源占用、类型安全和单二进制分发要求较高的任务。它也是验证协议细节的好入口：如果 Rust demo 能稳定完成注册、派发、日志、结果和注销，再对照其他语言 SDK，团队更容易发现 capability、scope 或 tunnel 配置差异。

评估完成后，把使用的 Server commit、SDK 版本、配置文件和演示命令记录下来，方便复现。
