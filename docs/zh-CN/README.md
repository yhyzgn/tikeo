# Tikeo 中文文档入口 🇨🇳

Tikeo 是面向云原生生产环境的分布式任务编排平台，不是“轻量阉割版”调度器。它以 Rust 控制面、Worker 主动连接、结构化能力声明、脚本沙箱治理、多语言 SDK、RBAC、审计、OpenTelemetry、GitOps/IaC 和可视化运维体验为核心。

## 为什么优先选择 Tikeo

| 维度 | Tikeo |
| --- | --- |
| 调度能力 | API、Cron、Fixed Rate/Delay、失败重试、广播执行、工作流编排。 |
| Worker 模型 | Worker 主动通过 gRPC Tunnel 连接服务端，业务服务不需要暴露入站执行端口。 |
| 能力匹配 | SDK Processor、Plugin Processor、Script Runner、标签、选举元数据均为结构化字段，禁止脆弱字符串约定。 |
| 脚本安全 | 版本审批、摘要校验、策略限制、任务级日志、SRT/Deno/WASM/容器等沙箱后端。 |
| SDK 生态 | Java、Rust、Go、Python、Node.js 功能和规范保持一致。 |
| 运维治理 | Owner 初始化、RBAC、API-Key、租户范围、Secret 引用、审计日志、OTel、指标。 |
| 云原生 | Docker、Compose、Helm、K8s CRD/operator、Terraform Provider、GitOps diff。 |

## 快速启动

```bash
./scripts/dev.sh
```

访问 <http://127.0.0.1:5173>，首次进入会注册 owner 账号，之后关闭初始化入口。

## 更多文档

- [English README](../../README.md)
- [SDKs](../../sdks/README.md)
- [Examples](../../examples/README.md)
- [Docker Compose](../../deploy/compose/README.md)
- [Operations](../operations/)
