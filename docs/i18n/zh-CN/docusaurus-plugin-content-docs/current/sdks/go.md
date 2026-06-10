---
title: Go Worker SDK
description: Go SDK 与 Worker demo 的验证入口。
---

# Go Worker SDK

Go SDK 位于 `sdks/go/tikeo`，可运行 Worker demo 位于 `examples/go/worker-demo`。它适合需要轻量静态二进制、Go 原生生态集成或平台侧工具链一致性的团队。

## 运行时要求

Go SDK 当前声明 Go 1.26+。仓库文档、`go.mod`、CI 和 runtime 徽章必须保持一致；如果降低或提高版本，应同时更新 SDK README 与文档站。

## 从 Go module proxy 安装

将 `${TIKEO_VERSION}` 替换为 README 顶部 `Go SDK` 徽标显示的版本号。Go 命令使用 tag 语法，因此需要写成 `v${TIKEO_VERSION}`。

```bash
go get github.com/yhyzgn/tikeo/sdks/go/tikeo@v${TIKEO_VERSION}
```

```go
import "github.com/yhyzgn/tikeo/sdks/go/tikeo"
```

## 验证 SDK

```bash
cd sdks/go/tikeo
go test ./... -count=1
```

## 验证 demo

```bash
cd examples/go/worker-demo
go test ./... -count=1
```


## Management API 创建并触发任务

Go management client 实现在 `sdks/go/tikeo/management.go`。它固定作用于一个 namespace/app，并使用 `x-tikeo-api-key` 鉴权，通常从 `TIKEO_MANAGEMENT_API_KEY` 读取；不要把人类 OIDC session 或 UI bearer token 塞进 SDK Worker。`APIJob` 创建 API 调度的 processor job，`APITrigger` 发送 `triggerType=api`，默认 `executionMode=single`。

```go
package main

import (
    "context"
    "os"

    tikeo "github.com/yhyzgn/tikeo/sdks/go/tikeo"
)

func createAndTrigger(ctx context.Context) error {
    endpoint := os.Getenv("TIKEO_MANAGEMENT_ENDPOINT")
    if endpoint == "" {
        endpoint = "http://127.0.0.1:9090"
    }
    client := tikeo.NewManagementClient(
        endpoint,
        os.Getenv("TIKEO_MANAGEMENT_API_KEY"),
        "dev-alpha",
        "orders",
    )

    created, err := client.CreateJob(ctx, tikeo.APIJob("go-echo-api", "demo.echo"))
    if err != nil {
        return err
    }
    instance, err := client.TriggerJob(ctx, created.ID, tikeo.APITrigger())
    if err != nil {
        return err
    }
    if instance.TriggerType != "api" || instance.ExecutionMode != "single" {
        panic("unexpected trigger response")
    }
    return nil
}
```

广播扇出必须显式选择。`BroadcastAPITrigger` 会序列化 `executionMode=broadcast` 与 `broadcastSelector`；保持它与单 Worker 默认触发分开，避免一次 API 调用误跑到所有匹配 Worker。

```go
broadcast := tikeo.BroadcastAPITrigger(&tikeo.BroadcastSelectorRequest{
    Tags:   []string{"manual-demo"},
    Region: "us-east-1",
    Labels: map[string]string{"worker_pool": "go-blue"},
})
_, err := client.TriggerJob(ctx, created.ID, broadcast)
```


## Source-backed 参考链接

SDK helper 文档必须锚定到从源码整理出的 API 与协议参考：

- 创建 helper 端点：[`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- 触发 helper 端点：[`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- 实例轮询端点：[`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- 实例日志端点：[`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker 派发消息：[`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## Worker Tunnel 模型

Go Worker 与 Rust、Java、Python、Node.js Worker 遵循同一协议：主动连接 Server，注册 metadata，心跳，接收派发，并通过 tunnel 回传日志和结果。

## 能力广告纪律

Go Worker 只能广告真实 runtime 支持的 structured processor 与 script capability。如果 sandbox runner 不可用，应提供安全错误边界，而不是把能力暴露给 Server 调度。

## 评估清单

- 在 SDK 和 demo 目录运行 `go test ./... -count=1`。
- 本地启动 Server 后，以 live mode 连接 Go Worker。
- 确认 Web 控制台能看到 session 与 capability snapshot。
- 触发路由到 Go processor 的任务。
- 检查日志、结果 payload 与审计证据。

## 打包建议

容器化 Go Worker 时，优先使用只包含 worker binary、配置和可信证书的小镜像。不要在镜像里混入不需要的构建工具或明文凭据。

## 适合场景

Go Worker 适合平台工程、基础设施自动化、网络服务集成和需要快速交付静态二进制的团队。评估时不要只看测试是否通过，还要看 worker pool、labels、structured capabilities 是否能让 Server 做出可解释的调度决策。
