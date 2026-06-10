---
title: Go Worker SDK
description: Go SDK 与 Worker demo 的 operator-grade 验收入口。
---

# Go Worker SDK

Go SDK 位于 `sdks/go/tikeo`，可运行 Worker demo 位于 `examples/go/worker-demo`。本文以 `config.go`、`management.go`、`client.go`、`grpc_client.go` 和 demo `main.go` 为事实来源。Go Worker 与其他语言一致，是 **outbound-only**：Worker 进程主动拨出到 Worker Tunnel，注册能力、心跳、接收 `DispatchTask`，再回传 task log 与 result；不要把业务 Worker 设计成 inbound HTTP Service。

## 依赖坐标

Go module 坐标来自 `sdks/go/tikeo/go.mod`：

```bash
go get github.com/yhyzgn/tikeo/sdks/go/tikeo@v${TIKEO_VERSION}
```

```go
import tikeo "github.com/yhyzgn/tikeo/sdks/go/tikeo"
```

仓库当前 SDK 包和 demo 都使用 module path `github.com/yhyzgn/tikeo/sdks/go/tikeo`。发布版本使用带 `v` 的 tag 语法；文档中的 `${TIKEO_VERSION}` 应与 README 徽标或发布流水线一致。Go SDK 导出 `WorkerConfig`、`LocalConfig`、`Client`、`TaskContext`、`TaskOutcome`、`TaskProcessorFunc`、`ManagementClient`、`APIJob`、`PluginAPIJob`、`ScriptAPIJob`、`APITrigger`、`BroadcastAPITrigger` 与 `BroadcastSelectorRequest`。

## WorkerConfig 默认值

`LocalConfig(endpoint, clientInstanceID)` 的源码默认值是：`Endpoint=endpoint`，`ClientInstanceID=clientInstanceID`，`Namespace="default"`，`App="default"`，`Name=clientInstanceID`，`Region="local"`，`Version="dev"`，`Cluster="local"`，`Labels=map[string]string{}`，`HeartbeatEvery=10*time.Second`。`Capabilities` 与 `Structured` 默认空。`Validate()` 会拒绝空 endpoint、client instance id、namespace、app、name、cluster，以及非正数 heartbeat。

Go demo 覆盖了 operator scope：`TIKEO_WORKER_ENDPOINT` 默认 `http://127.0.0.1:9998`，`TIKEO_WORKER_CLIENT_INSTANCE_ID` 默认 `go-worker-demo-local`，namespace/app 默认 `dev-alpha`/`orders`，cluster/region 默认 `local`，tag 包含 `go` 与 `manual-demo`，默认 SDK processors 是 `demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail`，label `worker_pool` 默认 `go-blue`。`TIKEO_ENABLE_PLUGIN_SQL` 按 `enabledByDefault` 处理，因此没有显式禁用时会广告 plugin type `sql` 与 processor `billing.sql-sync`。

## 最小 Worker

Go 最小 Worker 的职责是：配置一个 outbound client，声明真实处理器，连接 tunnel，循环处理任务。下面片段保留 demo 的关键结构，但不包含可选脚本 runner；如果 SRT、Deno、Docker、Podman 或自定义本地命令不可用，不要调用 `AddScriptRunner`。

```go
package main

import (
    "context"
    "log"
    "time"

    tikeo "github.com/yhyzgn/tikeo/sdks/go/tikeo"
)

func main() {
    config := tikeo.LocalConfig("http://127.0.0.1:9998", "go-worker-demo-local")
    config.Namespace = "dev-alpha"
    config.App = "orders"
    config.AddTag("go")
    config.AddSDKProcessor("demo.echo")

    client, err := tikeo.NewClient(config)
    if err != nil { log.Fatal(err) }
    processor := tikeo.TaskProcessorFunc(func(_ context.Context, task tikeo.TaskContext) (tikeo.TaskOutcome, error) {
        task.LogInfo("go echo started")
        return tikeo.TaskOutcome{Success: true, Message: "go demo echo processed"}, nil
    })

    for {
        session, err := client.Connect(context.Background())
        if err != nil { log.Printf("connect failed: %v", err); time.Sleep(2 * time.Second); continue }
        stopHeartbeat := session.StartHeartbeat(context.Background())
        _, err = session.ProcessNext(context.Background(), processor)
        stopHeartbeat()
        _ = session.Close()
        if err != nil { log.Printf("worker tunnel ended: %v", err) }
    }
}
```

生产上要保持 capability discipline：`AddSDKProcessor`、`AddScriptRunner`、`AddPluginProcessor` 的输出会影响调度，不能把未安装或不可执行的 runner 广告出去。任务内日志用 `TaskContext.LogInfo/LogError`，连接、注册和 sandbox 解析日志走进程日志。镜像中只放 worker binary、配置和证书；不要混入构建工具或明文密钥。

## Management API 与管理客户端凭证

Go management helper 位于 `sdks/go/tikeo/management.go`。`NewManagementClient(endpoint, apiKey, namespace, app)` 会 trim endpoint 末尾斜杠，空 namespace/app 默认 `default`，HTTP client timeout 为 30 秒。每个请求发送 `accept: application/json` 与 `x-tikeo-api-key`，有 body 时发送 `content-type: application/json`。凭证应从 `TIKEO_MANAGEMENT_API_KEY` 或 Secret store 注入；demo 中 `TIKEO_MANAGEMENT_CREATE_EXAMPLES` 分支使用 `TIKEO_HTTP_URL` 与 `TIKEO_API_KEY`，生产文档建议统一改为 management 专用环境变量，避免和 UI/人类 token 混用。

helper 行为：`APIJob` 写入 `scheduleType=api`、`processorName`、`enabled=true` 和默认 retry；`PluginAPIJob` 写入 `processorType`；`ScriptAPIJob` 写入 `scriptId`；`APITrigger()` 写入 `triggerType=api`、`executionMode=single`；`BroadcastAPITrigger(selector)` 写入 `triggerType=api`、`executionMode=broadcast` 与 `broadcastSelector`。

```go
ctx := context.Background()
client := tikeo.NewManagementClient(
    envOr("TIKEO_MANAGEMENT_ENDPOINT", "http://127.0.0.1:9090"),
    os.Getenv("TIKEO_MANAGEMENT_API_KEY"),
    "dev-alpha",
    "orders",
)
created, err := client.CreateJob(ctx, tikeo.APIJob("go-echo-api", "demo.echo"))
if err != nil { return err }
instance, err := client.TriggerJob(ctx, created.ID, tikeo.APITrigger())
if err != nil { return err }
if instance.TriggerType != "api" || instance.ExecutionMode != "single" { panic("unexpected trigger response") }

selector := &tikeo.BroadcastSelectorRequest{
    Tags: []string{"manual-demo"},
    Region: "us-east-1",
    Labels: map[string]string{"worker_pool": "go-blue"},
}
_, err = client.TriggerJob(ctx, created.ID, tikeo.BroadcastAPITrigger(selector))
```

参考锚点：[`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)、[`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)、[`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)、[`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)、[`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)。

## Demo 行为与脚本运行时

Go demo 默认尝试按 `TIKEO_WORKER_SCRIPT_LANGUAGES` 注册 `shell,python,javascript,typescript,powershell,php,groovy,rhai`。`TIKEO_WORKER_SCRIPT_SANDBOX=auto` 时，JS/TS 走 Deno，其它语言走 SRT；也支持 `docker`、`podman`、显式本地开发 runner，以及 `TIKEO_ENABLE_UNAVAILABLE_SCRIPT_ADAPTERS` 控制的不可用 adapter。`TIKEO_SANDBOX_AUTO_INSTALL` 被禁用时不会自动安装 sandbox 工具。验收时重点看 `scripts.AddCapabilities(&config)` 后的 structured capabilities 是否只包含实际注册成功的 runner。

## 源码事实索引与排错边界

核对 Go 集成时，先读 `sdks/go/tikeo/config.go` 的 `LocalConfig`、`Validate` 和 capability helper，再读 `grpc_client.go` 与 `client.go` 的 outbound tunnel、心跳和任务处理。`task.go` 规定 processor 返回 `TaskOutcome{Success, Message}`，并通过 `TaskContext.LogInfo/LogError` 写实例日志。`examples/go/worker-demo/main.go` 是最完整的运行样本：它先构造 registration，再按环境变量解析 plugin SQL、脚本语言、sandbox backend 和 dry-run/live 模式。排错顺序应是：确认配置 scope 一致，确认 `client.Registration()` 中的 structured processors、script runners、plugin processors 和 labels 正确，确认 Server 能看到 session，再触发 job。不要因为某个 runner 缺失就扩大广告范围；缺失 runner 应表现为不可调度或明确失败，而不是让任务在宿主机上无边界执行。

## 生产上线检查

上线前把 Go Worker 打包为独立进程或最小容器镜像，并把配置、证书和密钥交给部署系统注入。`ClientInstanceID` 是观测和重连提示，不能替代 Server ack 返回的 `worker_id`、generation 和 fencing token。多副本部署时，用 namespace、app、cluster、region、`worker_pool` label 和 tags 表达调度域；不要让多个环境共用同一 app scope。每次变更 capability 都应伴随发布记录：新增 SDK processor、plugin processor 或 script runner 都会改变 Server 可派发的任务集合。Management API key 只能用于控制面 job 创建和触发，禁止放入镜像、命令行历史或普通应用日志。

Go 的生产观测还应覆盖 tunnel 重连次数、heartbeat 延迟、任务成功率、失败分类和 management 请求错误率。发布后先用单副本验证，再逐步扩容同一 worker pool，确认调度分布、broadcast 选择器和故障重试都符合预期。

如果 Go Worker 还调用外部系统，建议把外部凭证与 Tikeo management 凭证分开轮换，并在任务日志中只记录可审计的业务标识，不记录完整请求体或响应体。

灰度期间保留 `TIKEO_WORKER_ONESHOT` 或等价单任务运行方式，便于验证一个 job instance 的完整生命周期。

## 现场验收 runbook

1. SDK 单元测试：`cd sdks/go/tikeo && go test ./... -count=1`。demo 测试：`cd examples/go/worker-demo && go test ./... -count=1`。
2. dry-run：设置 `TIKEO_WORKER_DRY_RUN=1` 或 `TIKEO_WORKER_CONNECT=0` 后运行 demo，确认输出 registration JSON 和 `dry_run_heartbeat_sequence`，并确认没有业务 inbound service。
3. live tunnel：启动 Server，设置 `TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998`、namespace/app、cluster/region、`TIKEO_WORKER_POOL=go-blue`，运行 demo，Web 控制台应出现 outbound Worker session 和 capability snapshot。
4. Management 验证：用 `ManagementClient` 创建 `APIJob("go-echo-api", "demo.echo")` 并 `APITrigger()`，抓包或 Server 日志确认 `x-tikeo-api-key` 来自 `TIKEO_MANAGEMENT_API_KEY`，响应 `triggerType=api`、`executionMode=single`。
5. 广播只在明确 fan-out 时执行：`BroadcastAPITrigger` 携带 tag `manual-demo` 与 label `worker_pool=go-blue`，核对只有目标 Worker 收到任务。
6. 故障演练：触发 `demo.fail`，确认任务失败、日志可见、Worker session 仍可继续；禁用脚本依赖后确认不可用 runner 不被广告。
