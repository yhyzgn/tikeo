---
title: Server、Worker 与 Worker Tunnel
description: Tikeo 区别于“Server 回调执行器”模型的主动出站 Worker Tunnel。
---

# Server、Worker 与 Worker Tunnel

Tikeo 最关键的运行时边界是 Worker Tunnel。

```text
Worker process  ── outbound gRPC/HTTP2 tunnel ──>  Tikeo Server
       ▲                                              │
       └──────── dispatch / cancel / logs / result ───┘
```

## 为什么主动出站很重要

业务 Worker 经常运行在 Kubernetes、私有 VPC、跨集群网络、NAT 或严格防火墙之后。Tikeo 不要求它们暴露入站执行端口。Worker 主动注册、心跳、接收派发、上报日志、返回结果，并通过同一条长连接完成注销。

## 身份与 fencing

Server 在注册阶段分配权威 worker identity。session generation 与 fencing token 用来阻止旧进程、断线重连残留或被替换的逻辑 Worker 继续写入结果。这比依赖 worker 名称约定更可靠，也便于审计。

## 运营可见性

Worker session 和 capability snapshot 会持久化。Server 重启后，系统仍能保留 worker 可见性证据，而不是完全依赖内存注册表。评估时应查看 Web Workers 页面、session history、transport error 与 lost reason。

## 安全边界

Server 负责调度、治理、状态、API 和审计。用户代码、动态脚本、HTTP 调用、SQL processor、plugin processor 和 sandbox runner 应在 Worker 或受控 runtime 中执行，不能塞进 Server 进程。

## 验证清单

不要只验证 TCP 连接。完整验证应确认：注册返回权威 worker id、心跳被接受、派发能到达 Worker、日志/结果包含 assignment token、旧 generation 结果会被拒绝、优雅注销能留下可见 session 事件。

## 部署含义

Worker 可以放在私有 Pod、VM、systemd 服务或另一个集群中。部署系统应该暴露 Server 的 Worker Tunnel 入口，但不应该创建业务 Worker 的任意入站执行 Service。
