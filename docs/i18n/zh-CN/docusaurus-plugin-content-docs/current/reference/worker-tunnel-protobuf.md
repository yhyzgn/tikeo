---
title: Worker Tunnel protobuf 参考
description: SDK 与服务端调度使用的 outbound Worker Tunnel gRPC/protobuf 契约参考。
---

# Worker Tunnel protobuf 参考

本页从 `crates/tikeo-proto/proto/worker.proto` 整理而来，该文件是
Worker Tunnel 的服务端 canonical protobuf 契约。各语言 SDK 会内置生成后
或复制后的绑定，但协议变更必须先从这个源文件开始。当前 package 是
`package tikeo.worker.v1`。

Worker Tunnel 是业务 Worker 进程主动出站连接到 Tikeo Server 的通道。Worker
不暴露入站端口。调度、取消、drain 等 Server → Worker 动作都写回已有 stream，
从而简化跨集群、跨 VPC、NAT 和 Kubernetes namespace 部署。

## Service surface

```protobuf
service WorkerTunnelService {
  rpc OpenTunnel(stream WorkerMessage) returns (stream ServerMessage);
  rpc SubscribeTaskLogs(SubscribeTaskLogsRequest) returns (stream TaskLog);
}
```

`WorkerTunnelService.OpenTunnel` 是长期双向 stream，用于注册、心跳、派发、
日志、任务结果、注销和 checkpoint。`SubscribeTaskLogs` 为需要历史回放加
实时更新的消费者提供任务日志流。

## 消息方向表

| 方向 | 消息 | 用途 |
| --- | --- | --- |
| Worker → Server | `RegisterWorker` | 发送 namespace、app、cluster、region、labels、旧式 capabilities、structured capabilities 与可选 election 设置。 |
| Worker → Server | `Heartbeat` | 使用 `worker_id`、generation、sequence 和 fencing token 续租。 |
| Worker → Server | `TaskLog` | 携带 `instance_id`、level、message、sequence 和 `assignment_token` 写入任务日志。 |
| Worker → Server | `TaskResult` | 使用 success、message 和 `assignment_token` 完成已分配任务。 |
| Worker → Server | `TaskCheckpoint` | 使用 `checkpoint_json` 为长运行任务保存可恢复进度。 |
| Worker → Server | `UnregisterWorker` | 优雅关闭权威 Worker session。 |
| Server → Worker | `WorkerRegistered` | 返回服务端分配的 `worker_id`、lease seconds、generation 和 fencing token。 |
| Server → Worker | `Ping` | 保持 stream 活跃并测量存活。 |
| Server → Worker | `DispatchTask` | 通过已有 outbound tunnel 向选中的 Worker 下发任务。 |

## RegisterWorker

`RegisterWorker` 携带 Worker 的逻辑 scope。可选 `client_instance_id` 只是客户端
稳定 hint；Tikeo 会在 `WorkerRegistered` 中分配权威 `worker_id`。新的路由
应优先使用 `structured_capabilities`，而不是旧式字符串 capabilities。Worker
集群选主使用 `WorkerClusterElection`，包含可选稳定 domain 和确定性 priority。

## WorkerRegistered 与 Heartbeat

`WorkerRegistered` 返回 Worker 后续消息必须回显的权威字段：`worker_id`、
`generation` 和 `fencing_token`。`Heartbeat` 携带相同 identity 数据以及递增
sequence。服务端用这些字段在重连、替换 session 或 lease 过期后拒绝陈旧 incarnation。

## DispatchTask

`DispatchTask` 是核心 Server → Worker 指令，包含：

- `instance_id` 与 `job_id`，指向已调度的执行记录。
- `payload` bytes，作为 processor 输入。
- `processor_name`，Java、Rust、Go、Python、Node.js 和未来 SDK adapter 使用的显式路由 key。
- `processor_binding`，用于动态脚本或 WASM 执行元数据。
- `assignment_token`，服务端签发的 authority，日志、checkpoint 和结果必须回显。

SDK 文档应把 processor helper 行为链接到该消息，因为 Worker 会按
`processor_name` 路由进入本地处理器，并用 `assignment_token` 证明 assignment
ownership。

## TaskLog、TaskResult 与 TaskCheckpoint

`TaskLog` 与 `TaskResult` 是 Worker 侧生成的执行证据。`TaskLog` 保存运维可见
进度，并可通过 `/api/v1/instances/{instance}/logs` 读取。`TaskResult` 把实例
推进到成功或失败终态。二者都包含 `assignment_token`；Worker 不能为未分配给
自己的任务伪造结果。

`TaskCheckpoint` 为长运行任务提供有序 `checkpoint_json` 快照，并使用与日志和
结果相同的 Worker identity 与 assignment-token 边界。

## 动态 processor binding

`TaskProcessorBinding` 可以包含 `ScriptProcessorBinding` 或 `WasmProcessorBinding`。
这些 binding 是不可变执行快照：脚本/模块 bytes、version id、SHA-256 完整性字段、
runtime 限制、network/file/env grant 和 sandbox backend 元数据。Server 通过
tunnel 分发快照；Worker 执行策略。Server 仍然不执行用户代码。

## 运维不变量

- Worker 进程主动发起 `OpenTunnel`；不要暴露业务 Worker 入站端口。
- 服务端分配的 `worker_id` 才是权威身份；`client_instance_id` 只是 hint。
- capabilities 必须描述真实可执行能力。不要广告缺失 runtime tool 的 script 或 plugin runner。
- 日志、checkpoint 和结果必须携带 `DispatchTask` 中的 `assignment_token`。
- HTTP 管理 API 负责观察和触发工作；真实执行证据通过 `WorkerTunnelService` 流动。
