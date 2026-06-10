---
title: 排障指南
description: Tikeo 本地评估失败时的第一组检查。
---

# 排障指南

## Server 无法启动

先运行：

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

检查配置解析错误、数据库连接错误、端口占用或 migration 失败。不要在原因未确认前修改业务代码。

## 健康检查失败

```bash
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/readyz
```

如果 `healthz` 失败，说明 Server 不可达；如果 `readyz` 失败，检查 storage、migration 或依赖 readiness 日志。

## Worker 不可见

- 确认 Worker 能访问 Worker Tunnel endpoint。
- 确认 Worker 广告的是真实 capability。
- 确认 generation/fencing token 没有拒绝陈旧心跳或结果。
- 在 Web 控制台查看 worker session history。

## Docker 镜像构建慢

Server 镜像验证会编译 Rust workspace，冷启动 GitHub runner 上明显慢于 Web 镜像验证。这不一定是失败，应先看日志是否仍在正常编译。

## 推荐排查顺序

1. 确认进程运行。
2. 确认 `healthz` / `readyz`。
3. 查看 storage migration 日志。
4. 查看 Worker Tunnel 可达性。
5. 查看 worker generation/fencing token 拒绝信息。
6. 查看 Web API 响应与浏览器 console。
7. 查看 audit 与 instance logs 中的脚本治理或策略失败。

## 常见路由失败

如果没有在线 Worker 广告所需 capability，任务可能停留 pending。不要用宽泛 wildcard capability 修复；应修正 processor binding、script backend、worker pool assignment 或 worker runtime 安装。

## 升级问题时带什么证据

报告问题时请包含 Server commit、config 文件路径、数据库后端、Worker SDK 语言/版本、health/readiness 输出，以及相关 instance id 或 audit id。

## 保留现场

排障时优先保留日志、配置路径、命令输出和相关 ID。不要先清空数据库或重启所有服务，否则会丢掉最有价值的 migration、fencing、policy、attempt 和 audit 证据。能复现的问题应固化为脚本或测试。
