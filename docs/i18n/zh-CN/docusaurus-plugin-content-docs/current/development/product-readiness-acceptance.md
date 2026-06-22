---
title: 产品就绪验收清单
description: 通知中心、tikeo-migrate 迁移 CLI 和 Raft FSOD Server HA 的验收与发布就绪清单。
keywords: [tikeo 验收, 通知中心, tikeo migrate, raft fsod, 发布就绪]
---

# 产品就绪验收清单

当维护者、发布负责人或接手人需要判断“通知中心、旧调度器迁移 CLI、Raft FSOD Server HA 这三块能不能交付下一轮开发/验收”时，使用本页。本页不替代功能手册，而是把跨功能验收门槛、证据命令和生产风险串起来。

## 范围与状态

| 模块 | 当前就绪状态 | 规范文档入口 | 主要证据 |
| --- | --- | --- | --- |
| 通知中心 | 已可做本地/预发验收；启用的每类渠道至少需要一次真实 provider 测试并归档投递证据。 | [通知用户指南](../user-guide/notifications)、[通知中心参考](../reference/notification-center)、[配置参考](../reference/configuration#通知中心投递) | 渠道测试结果、policy materialization、`notification_delivery_attempts`、消息 trace、脱敏 API 响应。 |
| 旧调度器迁移 CLI | 可作为 review-first 迁移助手；`plan` 不产生副作用，`apply` 只做本地旧 Worker 项目原地迁移改造；预发导入是单独复核后的控制台/API/GitOps 动作。 | [旧调度器迁移指南](../integrations/migrating-from-legacy-schedulers) | `.tikeo-migration/manifest.json`、`jobs.tikeo.md`、`data-import-plan.json`、`CHECKLIST.md`、`code-apply-evidence.json`、已复核导入证据。 |
| Raft FSOD Server HA | 可做本地 Kind 和预发验收；需要外部 DB、稳定 StatefulSet 身份、Raft transport token 和 gRPC/HTTP2 Worker Tunnel 链路。 | [Server 高可用与 Raft FSOD 集群](../deployment/server-ha)、[Kubernetes 与 Helm](../deployment/kubernetes)、[生产部署](../deployment/production) | `scripts/verify-raft-ha-rollout.sh`、`scripts/kind-raft-ha-e2e.sh`、`scripts/cloud-raft-ha-acceptance.sh`、cluster diagnostics、FSOD DB 快照、failover 前后实例结果。 |
| 跨语言 Worker soak | 可选的 release-candidate 运行时门禁，用于验证 Go/Rust/Python/Node 多轮派发、任务日志和 queue/outbox metrics。 | [SDK 与 API 集成](../integrations/sdk-and-api)、[Worker 用户指南](../user-guide/workers) | 手动 workflow `.github/workflows/release-candidate-worker-soak.yml`、`TIKEO_CROSS_SOAK_SECONDS=120 deploy/smoke/cross-language-worker-parity-smoke.sh`、`cross-language-worker-soak` artifact、`*-soak-summary.json`、`*-soak-summary.csv`、`*-soak-metrics.jsonl`。 |

停止条件不是“文档写完”，而是证据齐全：每个通过项都应该有可复现命令或 UI 动作、检查的路由/文件、观察到的状态，以及证据目录或产物路径。

如果要一次性生成本地交接证据包，执行：

```bash
./scripts/release-readiness-evidence.sh
```

聚合脚本会写入 `.dev/reports/release-readiness-evidence-*/REPORT.md` 和各模块 `summary.json`。它通过协议真实的 loopback provider 证明通知中心投递链路，演练完整 `tikeo-migrate` 旧项目迁移链路并输出旧 Worker 项目原地迁移，并在提供 `TIKEO_CLOUD_HA_SERVER_URL` 时执行真实云 HA 探针；未提供云目标时，会明确输出云环境延期边界报告。

## 通知中心验收

通知中心验收要证明：渠道配置、模板渲染、策略物化、retry/DLQ 和脱敏能闭环工作。

| 门槛 | 验收动作 | 保留证据 |
| --- | --- | --- |
| 渠道配置 | 在 Web 抽屉中为当前环境启用的 provider family 创建或编辑渠道：webhook-compatible、聊天机器人、PagerDuty、邮箱。webhook URL、signing secret、routing key、SMTP host/port/user/password/from 等配置都属于渠道行，保存后无需重启。 | 截图或 API 响应：`targetConfigured=true`、`targetRedacted` 已脱敏、`configJson` 不含原始密钥。 |
| 测试按钮 | 使用列表行 **测试** 和抽屉内 **测试**。成功时显示 provider 响应；失败时显示 HTTP/status/error body，而不是空响应 JSON 解析异常。 | 至少一条成功和一条安全失败的测试响应 JSON 或 UI 详情。 |
| 模板覆盖 | 对支持非文本模板的 provider 至少验证一条：Slack Block Kit、DingTalk action/feed card、Feishu interactive card、WeCom template card、PagerDuty incident、webhook JSON body、email subject/body。 | `/api/v1/notification-templates/{id}/render` 预览和实际投递 trace。 |
| 策略物化 | 将 policy 绑定到 job instance event，并触发作业进入目标状态。 | `notification_messages` 行或 API 摘要，包含 `policyId`、`eventType`、`resourceId`，使用模板时包含 `payload.template`。 |
| 投递队列 | 确认 delivered、retry、dead-letter 路径可见。 | 通知中心 delivery tab、`notification-delivery-attempts:queue-status` 或 DB 快照，包含 `retry_pending`、`retry_consumed`、`dead_letter` 和 delivered 行。 |
| 脱敏 | 获取 channel summary 和 message trace。 | webhook URL、signing secret、SMTP password、routing key、auth headers、URL path/query 不出现在响应或日志中。 |

建议本地验证：

```bash
./scripts/notification-provider-e2e-smoke.sh
python3 .github/tests/docs_site_contract_test.py
python3 .github/tests/demo_seed_topology_contract_test.py
cargo test -p tikeo-server notification --all-features
```

`notification-provider-e2e-smoke.sh` 会启动本地 Server 和 mock HTTP provider，发送一条成功测试通知和一条强制 provider 失败通知，然后验证 provider 收包、`notification_messages`、delivery attempts、queue 聚合、dead-letter 状态和目标脱敏。它是本地协议级证据，不替代具体租户中的 Slack/飞书/钉钉/企微/PagerDuty/SMTP 生产签核。

如果当前环境不能访问真实 provider，只能把 provider 投递门槛标记为 deferred；渲染、校验、脱敏和队列证据仍然必须保留。没有真实 outbound 结果时，不要声称该 provider 已生产就绪。

## 迁移 CLI 验收

`tikeo-migrate` 的定位是保守迁移助手：减少迁移工作量，但必须让人先复核语义差异，再导入数据或调整代码。

| 门槛 | 验收动作 | 保留证据 |
| --- | --- | --- |
| 自动探测 | 在旧 Java/Spring Worker 根目录执行 `tikeo-migrate plan`，自动识别 XXL-JOB/PowerJob 依赖和源码、Spring Boot major、可发现的 datasource 和调度器表。 | CLI 输出，以及 `.tikeo-migration/manifest.json` 中的 framework、DB source、Java project plan。 |
| `plan` 无副作用 | 确认 `plan` 不改旧源码、不调用 Tikeo Server。 | `plan` 前后 `git diff` 干净，新增文件仅在 `.tikeo-migration/` 或指定输出目录。 |
| 数据复核 | 复核生成的 job draft 和语义警告。 | `jobs.tikeo.md`、`data-import-plan.json`，以及 `ready`、`needs_review`、`skipped` 数量。 |
| 代码迁移建议 | 复核 Java 依赖和 processor patch 建议。 | 迁移分支上的 `java-project-plan.md`、`.json` 和 `java-patches/*.patch`。 |
| 本地 apply | 执行 `tikeo-migrate apply --bundle ./.tikeo-migration`；编译/测试迁移后的项目并检查生成配置占位符。 | `code-apply-evidence.json`、`CODE_MIGRATION_REPORT.md`、迁移后源码 diff，以及追加了 Tikeo 占位配置的原旧调度器配置文件。 |
| 预发 live import | 只导入已复核的 `ready` job，启动匹配的 Tikeo Worker，并触发至少一个迁移作业。 | Tikeo job id、实例结果/日志、Worker processor name，以及和旧行为的对比。 |
| Release 产物 | 确认 release 中有各平台 `tikeo-migrate` 压缩包和 checksum。 | GitHub Release 中 Linux、macOS Intel、macOS Apple Silicon、Windows 资产列表。 |

建议验证命令：

```bash
./scripts/migration-cli-full-chain-smoke.sh
cargo test -p tikeo-migrate
python3 .github/tests/workflow_contract_test.py
python3 scripts/check-source-size.py
```

`migration-cli-full-chain-smoke.sh` 会创建临时旧 Spring Boot + XXL-JOB 项目，写入本地旧调度器 DB，执行零参数 `tikeo-migrate plan`，验证生成的迁移包，执行在旧 Worker 项目中本地原地 `apply`，检查就地配置占位符，并归档`reviewed-import-payloads.json`。

迁移验收不是“所有 job 都被导入”就结束。真正通过的标准是：不兼容语义有明确处理决定，导入到预发的 job 能被触发执行，并且存在支持回滚的 dual-run / cutover / disable legacy 计划。

## Raft FSOD Server HA 验收

Server HA 验收要证明系统没有把正确性藏在 Pod 内存中。API/Web 流量可以落到不同 Pod，Worker 可以连接另一个 gateway Pod，派发仍然通过 Raft fencing、shard ownership、durable outbox 和 assignment-token validation 完成。

| 门槛 | 验收动作 | 保留证据 |
| --- | --- | --- |
| 部署形态 | 多 Pod Server 使用 StatefulSet、headless peer DNS、外部 DB、一致 shard 配置和 Raft transport token。 | 渲染后的 Helm manifest 或 `kubectl get statefulset,svc,secret`。 |
| 调度 fencing | 只有一个节点报告 `canSchedule=true`；过期 term/token fail closed。 | `/api/v1/cluster`、`/api/v1/cluster/diagnostics` 和 `scripts/verify-raft-ha-rollout.sh` 输出。 |
| Shard ownership | `cluster_shard_ownership` 存在 active 行，skew 在阈值内，map version/count 一致。 | DB 快照、metrics summary、`shardOwnership` diagnostics。 |
| 持久化派发 | `worker_dispatch_outbox` 在 stream 投递前记录派发意图；gateway 或 Worker 重连后 queued/delivered 行能恢复。 | failover 前后 FSOD DB 快照、outbox metrics、Worker 日志。 |
| 跨 Pod API/Web 读取 | 业务 API 读取共享持久状态，而不是 Pod 本地内存。 | 通过 Service 重复请求时，job/instance/message 状态保持一致。 |
| API Pod 与 Worker gateway 不同 | 通过 Pod A 触发作业，Worker stream 保持在 Pod B。 | diagnostics 显示 local/remote Worker 数、gateway node id 和成功实例结果。 |
| Leader failover | 删除或重启当前 Leader；新 Leader 投影 ownership，failover 后触发的作业完成。 | Kind/预发 fault-drill 报告、前后实例结果、Kubernetes events。 |
| 网络链路 | Worker Tunnel 支持 gRPC/HTTP2、idle timeout、TLS/mTLS；SSE dashboard 与 gRPC 配置分开验收。 | Ingress/Gateway/LB 配置，以及 [SSE 实时部署](../deployment/sse-realtime) 检查。 |

本地 Kind 验收：

```bash
TIKEO_KIND_E2E_KEEP=0 TIKEO_KIND_E2E_REBUILD_SERVER=1 scripts/kind-raft-ha-e2e.sh
```

预发 rollout gate：

```bash
TIKEO_SERVER_URL="https://tikeo.example.com" TIKEO_MANAGEMENT_API_KEY="$TIKEO_MANAGEMENT_API_KEY" TIKEO_EXPECTED_SERVER_REPLICAS=3 TIKEO_MAX_SHARD_SKEW=1 scripts/verify-raft-ha-rollout.sh
```

只读云环境验收探针：

```bash
TIKEO_CLOUD_HA_SERVER_URL="https://tikeo.example.com" TIKEO_CLOUD_HA_EXPECTED_REPLICAS=4 TIKEO_CLOUD_HA_WORKER_TUNNEL_HOST="worker-tunnel.example.com" scripts/cloud-raft-ha-acceptance.sh
```

Kind 可以验证本地 Kubernetes 语义，但不能替代云环境中的多可用区节点故障、托管 LB 行为、WAF/gateway idle timeout、TLS 证书和外部数据库 HA 验收。没有云目标时，`scripts/release-readiness-evidence.sh` 会把这项记录成云环境边界报告，而不是静默标记通过。

## 跨功能发布门槛

交给下一位 owner 或发布新版本前，至少收集一份短证据包：

| 项目 | 必要产物 |
| --- | --- |
| 版本与提交 | `git rev-parse HEAD`、release tag、release asset 列表。 |
| 文档 | README 链接、docs sidebar 入口、docs search/LLM 入口和本清单。 |
| 通知中心 | Provider 测试证据、message trace、retry/DLQ 快照、脱敏检查。 |
| 迁移 CLI | 示例 `.tikeo-migration` bundle、本地 apply 证据、已复核导入 payload、release assets。 |
| HA | Kind 或预发 HA report 目录、rollout gate 输出、failover 实例结果。 |
| 回归检查 | `docs_site_contract_test.py`、相关 Rust/package 测试、source-size check、`git diff --check`。 |

## 剩余风险与下一步

- 真实云环境 HA 仍需要按环境验证 ingress class、LB/WAF 行为、TLS/mTLS、NetworkPolicy 和托管数据库 HA。
- Provider 投递行为可能受租户策略影响；每个部署环境都应保留真实 Slack/DingTalk/Feishu/WeCom/PagerDuty/email 证据。
- 旧调度器语义等价性与业务域相关。路由、阻塞、并发、脚本语义应该保持 review-required，而不是静默自动转换。
- Release asset 可用性应在 pipeline 上传完成后，从真实 GitHub Release 页面核对。

## 前置条件

使用干净工作区、正在运行的本地或预发 Server，并把凭证放在环境变量中。HA 检查需要外部数据库和稳定 Kubernetes 身份。通知检查应使用测试安全的 provider 目标。

## 验收

有效证据包包含命令或 UI 动作、检查的路由/文件、观察到的状态和产物路径。最低限度执行：

```bash
python3 .github/tests/docs_site_contract_test.py
python3 scripts/check-source-size.py
git diff --check
```

## 故障排查

如果某个清单项失败，不要降低验收标准。按对应规范页面排查，保留失败响应和日志窗口，修复代码/配置/文档漂移，然后重新运行能证明该 claim 的最小检查。

## 生产检查清单

- [ ] 每个启用的通知 provider 都有真实测试记录，或明确的环境例外说明。
- [ ] 每个迁移 job 都已接受、带原因延期，或决定手工重建。
- [ ] 多 Pod Server HA 使用外部 DB 和稳定 Pod 身份下的 Raft FSOD，而不是 standalone 副本数。
- [ ] 验收证据存放在可归档目录中，不依赖临时 shell history。
- [ ] 环境相关风险已在生产 rollout 前分配 owner。
