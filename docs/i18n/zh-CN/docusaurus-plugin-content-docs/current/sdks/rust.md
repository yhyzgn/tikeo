---
title: Rust Worker SDK
description: Rust 依赖、最小 Worker、异常处理和 Management client 写法。
---

# Rust Worker SDK

先读 [SDK 与 API 集成指南](../integrations/sdk-and-api)。本文只说明 Rust 特有的依赖、最小 Worker、异常捕获和 Management client 写法。Rust SDK 位于 `sdks/rust/tikeo`；demo 位于 `examples/rust/worker-demo`。

## 前置条件

| Requirement | Value |
| --- | --- |
| Crate | `tikeo` |
| 版本占位符 | `${TIKEO_VERSION}`，来自 README 顶部包徽标或 release tag。 |
| Rust edition | `2024` |
| Rust baseline | `1.95` |
| Optional feature | `wasm` 启用 `wasmtime` |
| 主要 runtime deps | `tonic`, `prost`, `tokio`, `reqwest`, `serde`, `sha2`, `tracing` |

安装发布版：

```bash
cargo add tikeo@${TIKEO_VERSION}
```

验证仓库内 SDK：

```bash
cargo fmt --manifest-path sdks/rust/tikeo/Cargo.toml -- --check
cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
```

## 最小 Worker

`WorkerConfig::local(endpoint, client_instance_id)` 将 namespace/app 默认成 `default`，cluster/region 默认成 `local`，labels 和 structured capabilities 为空。只添加 Worker 真正能运行的 processor。

```rust
use async_trait::async_trait;
use tikeo::{install_task_log_bridge, TaskContext, TaskOutcome, TaskProcessor, WorkerClient, WorkerConfig, WorkerSdkError};

struct Echo;

#[async_trait]
impl TaskProcessor for Echo {
    async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError> {
        tracing::info!(processor = %task.processor_name, instance = %task.instance_id, "rust echo processor");
        Ok(TaskOutcome::Success("rust echo processed".to_owned()))
    }
}

#[tokio::main]
async fn main() -> Result<(), WorkerSdkError> {
    let _ = install_task_log_bridge();
    let mut config = WorkerConfig::local("http://127.0.0.1:9998", "rust-worker-1");
    config.namespace = "sdk-smoke".to_owned();
    config.app = "management".to_owned();
    config.add_sdk_processor("demo.echo");
    config.labels.insert("worker_pool".to_owned(), "rust-blue".to_owned());

    let client = WorkerClient::new(config);
    let mut session = client.connect().await?;
    loop {
        session.process_next(&Echo).await?;
    }
}
```

## 异常捕获

| Case | Rust 行为 |
| --- | --- |
| 预期业务失败 | 返回 `TaskOutcome { success: false, message }`；instance 记录失败，job retry policy 可重试。 |
| Worker/client 失败 | 返回或传播 `WorkerSdkError`；退出或重连前记录上下文。 |
| 不支持的 processor | 返回 failed `TaskOutcome`，不要广告未实现 processor。 |
| Task logs | 优先使用 `tracing::info!/warn!/error!` + `install_task_log_bridge()`；`TaskContext::log_info/log_error` 仅作为底层 fallback。 |

## Management client 写法

```rust
use tikeo::{ManagementBroadcastSelectorRequest, ManagementClient, ManagementCreateJobRequest, ManagementTriggerJobRequest};

#[tokio::main]
async fn main() -> Result<(), tikeo::WorkerSdkError> {
    let client = ManagementClient::new(
        "http://127.0.0.1:9090",
        std::env::var("TIKEO_MANAGEMENT_API_KEY").expect("TIKEO_MANAGEMENT_API_KEY"),
        "sdk-smoke",
        "management",
    );

    let job = client.create_job(ManagementCreateJobRequest::api("rust-echo-api", "demo.echo")).await?;
    let instance = client.trigger_job(&job.id, ManagementTriggerJobRequest::api()).await?;
    let broadcast = ManagementTriggerJobRequest::broadcast_api(Some(ManagementBroadcastSelectorRequest {
        tags: None,
        region: None,
        cluster: None,
        labels: Some(std::collections::HashMap::from([("worker_pool".to_owned(), "rust-blue".to_owned())])),
    }));

    println!("instance={} triggerType=api executionMode=single", instance.id);
    println!("broadcastSelector={:?}", broadcast.broadcast_selector);
    Ok(())
}
```

`ManagementClient::new` 发送 `x-tikeo-api-key`，trim endpoint，空 namespace/app 默认 `default`，并把 helper payload 映射到这些 anchor：

- Create helper → [`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- Trigger helper → [`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- Instance polling → [`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- Log inspection → [`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker dispatch → [`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## 验收

| Check | Command or evidence |
| --- | --- |
| SDK tests | `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features` |
| Worker registration | Worker 带 `demo.echo` 和 `worker_pool=rust-blue`。 |
| API trigger | Instance 显示 `triggerType=api` 和 `executionMode=single`。 |
| Worker logs | Instance logs 包含 `rust echo processed` 或 processor-specific log。 |

## 故障排查

| 现象 | 修复 |
| --- | --- |
| Management request unauthorized | 检查 `TIKEO_MANAGEMENT_API_KEY` 和 `x-tikeo-api-key`。 |
| 没有 Worker 被选中 | 对齐 namespace/app、processor name、`worker_pool` label。 |
| Broadcast 范围太宽 | 使用 `ManagementTriggerJobRequest::broadcast_api` 和 `ManagementBroadcastSelectorRequest`。 |
| Worker 因 tunnel error 退出 | 在 demo 式 `WorkerClient` loop 外加 supervisor 和 reconnect policy。 |

## 生产检查清单

- [ ] `WorkerConfig` 来自部署配置，不写死本地值。
- [ ] Worker 只广告已实现 processor。
- [ ] Management key 从 `TIKEO_MANAGEMENT_API_KEY` 或 secret manager 加载。
- [ ] 非幂等 processor 在使用 `ManagementCreateJobRequest::api` 前审查默认 retry policy。
- [ ] Broadcast trigger 必须审查 selector。


## 统一配置参数与默认值

不同语言 SDK 的代码写法不同，但接入 Tikeo 时面对的是同一组语义。不要把这些参数理解成各语言私有字段；它们最终都会进入 Worker Tunnel 注册、任务派发、Management API 创建任务和实例触发链路。

| 参数 | 默认值 | 生产建议 |
| --- | --- | --- |
| `endpoint` | 本地 Worker Tunnel 通常是 `http://127.0.0.1:9998` | 生产必须指向 Server 暴露的 Worker Tunnel 地址，并与 TLS/mTLS 配置一致。 |
| `namespace` | `default` 或示例中的 `sdk-smoke` | 每个团队、租户或环境应使用清晰命名，不要把生产任务混进 default。 |
| `app` | `default` 或示例中的 `management` | 与 Management API Key 的 app scope 保持一致。 |
| `clientInstanceId` | 示例手工指定 | 生产中应唯一且稳定，便于 Worker 页面和审计定位。 |
| `cluster` / `region` | `local` | 多机房部署必须真实填写，广播和选择器会使用这些信息。 |
| `labels` | 空 map | 用 `worker_pool`、`region`、`cluster` 等标签表达调度边界。 |
| `sdkProcessors` | 空列表 | 只声明当前进程真实实现的 processor，避免实例被派发后失败。 |
| `heartbeat` | 约 10 秒 | 保持默认即可；高延迟网络再根据运维策略调整。 |

## 管理客户端凭证

Management client 使用应用级 API Key，不使用浏览器里的人工登录 token。创建 key 时要绑定 namespace/app，运行时通过 `TIKEO_MANAGEMENT_API_KEY` 注入。所有语言的请求都会发送 `x-tikeo-api-key`，创建任务时应明确 `triggerType=api`、`executionMode=single`，需要广播时再设置 `broadcastSelector`。

| 决策 | 推荐做法 | 风险 |
| --- | --- | --- |
| API Key 保存位置 | Secret Manager、Kubernetes Secret 或 CI secret | 不要写进代码、README、截图或 shell 历史。 |
| 权限范围 | app-scoped service account | 不要用 Owner 或全局管理账号跑 SDK。 |
| 轮换 | 发布窗口内双写新旧 key | 直接删除旧 key 会让 Worker 或自动化立即失败。 |
| 验证 | 先创建 API 手动触发任务，再触发一次 | 只构建通过不能证明 Management API 可用。 |

## 现场验收 runbook

1. 确认 Server `/readyz` 通过，Web 控制台能看到目标 namespace/app。
2. 使用当前语言启动一个只声明 `demo.echo` 的 Worker。
3. 在 Worker 页面确认 `clientInstanceId`、region、cluster、labels 和 processor 列表正确。
4. 使用 Management client 创建 API 触发任务，确认返回 job id。
5. 触发一次 single instance，进入 Instances 页面查看状态、Worker、日志和 result。
6. 如果要验证广播，设置 `broadcastSelector`，确认多个符合标签的 Worker 都生成 attempt 或广播实例证据。
7. 制造一次业务失败和一次运行时异常，确认日志中能看到 message、stack 或错误路径。
8. 给失败事件绑定通知渠道，确认消息中的实例 ID、时间、状态、操作人、执行类型可以追溯。

## 故障排查表

| 现象 | 可能原因 | 处理方式 |
| --- | --- | --- |
| Worker 页面看不到进程 | endpoint/TLS/mTLS 或 token 不匹配 | 先看 Worker 启动日志，再看 Server Worker Tunnel 日志。 |
| 实例一直等待 | processorName、标签或 region/cluster 不匹配 | 对照 Jobs 页和 Workers 页的 capability。 |
| 触发 API 返回 401/403 | `TIKEO_MANAGEMENT_API_KEY` 无效或 scope 不对 | 重新创建 app-scoped key，确认 header 是 `x-tikeo-api-key`。 |
| 执行失败但没有日志 | processor 异常未被 SDK 捕获或进程崩溃 | 升级 SDK，确保 task log API 被调用，并查看 Worker 本地日志。 |
| 广播没有命中目标 | `broadcastSelector` 标签与 Worker labels 不一致 | 先用单实例验证，再逐步加 selector。 |

## 生产检查清单

- [ ] 依赖坐标固定到发布版本，而不是随意使用本地源码路径。
- [ ] WorkerConfig 默认值已经被生产环境显式覆盖。
- [ ] 最小 Worker 在目标环境成功注册并展示能力。
- [ ] 管理客户端凭证来自 Secret，不来自人工账号。
- [ ] 现场验收 runbook 的创建、触发、日志、失败、通知链路均通过。
