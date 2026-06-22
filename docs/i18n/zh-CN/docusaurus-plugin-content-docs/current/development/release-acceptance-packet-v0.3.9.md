---
title: v0.3.9 发布验收包
slug: release-acceptance-packet-v0.3.9
description: v0.3.9 发布与紧随其后的跨语言 Worker soak gate 的交接证据包。
keywords: [tikeo v0.3.9, 发布验收, 交接, raft fsod, worker soak]
---

# v0.3.9 发布验收包

接手 `v0.3.9` 之后的开发时使用本页。本页记录已经发布的内容、本地保留的证据、GitHub workflow 产物，以及仍需要真实云环境才能完成的生产检查。

## 版本边界

| 项目 | 值 |
| --- | --- |
| Release tag | `v0.3.9` |
| Release 页面 | [v0.3.9 release](https://github.com/yhyzgn/tikeo/releases/tag/v0.3.9) |
| Release 状态 | 已发布，非 draft，非 prerelease |
| 已观察到的资产数量 | `31` 个 uploaded assets |
| Release 提交证据 | `ee895ba7 chore: close ha follow-up gates` |
| 最新 main 追加提交 | `affb4605 test: add cross-language worker soak gate` |

`affb4605` 是 release 后的追加收尾提交。它把可复用的 release-candidate soak gate 和文档契约补到了 main；除非后续 tag 包含该提交，否则不要声称这些脚本改动已经包含在 `v0.3.9` 二进制里。

## 已观察到的 Release 资产

最后一次交接检查时，`v0.3.9` GitHub Release 包含以下已上传资产族：

| 资产族 | 已观察文件 |
| --- | --- |
| SDK 压缩包 | `go-sdk-0.3.9.tar.gz`、`java-sdk-0.3.9.tar.gz`、`nodejs-sdk-0.3.9.tar.gz`、`python-sdk-0.3.9.tar.gz`、`rust-sdk-0.3.9.tar.gz` |
| Server 二进制 | `tikeo-server-0.3.9-aarch64-apple-darwin.tar.gz`、`tikeo-server-0.3.9-x86_64-apple-darwin.tar.gz`、`tikeo-server-0.3.9-x86_64-pc-windows-msvc.zip`、`tikeo-server-0.3.9-x86_64-unknown-linux-gnu.tar.gz` |
| 迁移 CLI 二进制 | `tikeo-migrate-0.3.9-aarch64-apple-darwin.tar.gz`、`tikeo-migrate-0.3.9-x86_64-apple-darwin.tar.gz`、`tikeo-migrate-0.3.9-x86_64-pc-windows-msvc.zip`、`tikeo-migrate-0.3.9-x86_64-unknown-linux-gnu.tar.gz` |
| Operator 二进制 | `tikeo-operator-0.3.9-darwin-amd64.tar.gz`、`tikeo-operator-0.3.9-darwin-arm64.tar.gz`、`tikeo-operator-0.3.9-linux-amd64.tar.gz`、`tikeo-operator-0.3.9-linux-arm64.tar.gz`、`tikeo-operator-0.3.9-windows-amd64.zip` |
| Terraform provider | `terraform-provider-tikeo_v0.3.9_darwin_amd64.tar.gz`、`terraform-provider-tikeo_v0.3.9_darwin_arm64.tar.gz`、`terraform-provider-tikeo_v0.3.9_linux_amd64.tar.gz`、`terraform-provider-tikeo_v0.3.9_linux_arm64.tar.gz`、`terraform-provider-tikeo_v0.3.9_windows_amd64.exe.zip` |
| 部署与 Web 包 | `tikeo-0.3.9.tgz`、`tikeo-deploy-sources-0.3.9.tar.gz`、`tikeo-web-dist-0.3.9.tar.gz`、Compose YAML、Kubernetes manifest、CRD manifest |

## Workflow 证据

| Workflow 组 | 已观察结果 |
| --- | --- |
| Release / GitHub assets for `v0.3.9` | ✅ success |
| Publish / Java SDK | ✅ success |
| Publish / Python SDK | ✅ success |
| Publish / Node.js SDK | ✅ success |
| Publish / Go SDK | ✅ success |
| Publish / Rust SDK | ✅ success |
| Publish / Docker docs | ✅ success |
| Publish / Docker web | ✅ success |
| Publish / Docker server | ✅ success |
| Main `Coverage` for `affb4605` | ✅ success |
| Main `CI` for `affb4605` | 创建本页时仍在运行；继续合并前请以最新 GitHub Actions 结果为准。 |

后续发布前重新执行：

```bash
gh run list --branch main --limit 20
gh release view v0.3.9 --json tagName,url,isDraft,isPrerelease,assets
```

## 本地 HA 证据

最新本地 HA 验收使用多节点 Kind 集群，并通过必需 Pod anti-affinity 在单台开发机上逼近生产故障域。

| 信号 | 结果 |
| --- | ---: |
| HA confidence index | `99/100` |
| Server replicas / Kind worker nodes | `4 / 4` |
| Server Pod spread | gateway force-delete 前后均为 `4 / 4` 个不同 Kind worker nodes |
| Raft shard ownership | `64` active rows，`4` owners，rollout gate 中 ownership skew 为 `0` |
| Epoch fencing | `100/100`，stale owner token 拒绝由单测覆盖，并通过 Leader Pod 删除恢复验证 |
| Worker gateway reroute | `100/100`，旧 gateway 被 force-delete，Worker 经新 gateway 重连，并观察到 outbox reroute |
| Web/API Service load balancing | `96` 次集群内请求，覆盖 `4 / 4` 个 Server Pod，coverage ratio `1.0`，distribution index `94/100` |
| Evidence completeness | `26` 个 passed cases，`0` 个 failed cases |

本地报告仓库路径：`design/reports/kind-raft-ha-e2e-20260622.md`

本地复现：

```bash
TIKEO_KIND_E2E_KEEP=0 \
TIKEO_KIND_E2E_REBUILD_SERVER=1 \
scripts/kind-raft-ha-e2e.sh
```

## 跨语言 Worker soak 追加验证

release 后的 main 提交 `affb4605` 在 `deploy/smoke/cross-language-worker-parity-smoke.sh` 中加入了可重复执行的跨语言 Worker soak gate。后续 workflow `.github/workflows/release-candidate-worker-soak.yml` 把它暴露为手动 release-candidate gate，并支持 `ref`、`soak_seconds`、`soak_interval_seconds`、`rebuild_server`、`skip_web` 输入。普通 CI 默认不启用，也可以在本地显式运行：

```bash
TIKEO_CROSS_SKIP_WEB=1 \
TIKEO_CROSS_REBUILD_SERVER=0 \
TIKEO_CROSS_SOAK_SECONDS=120 \
TIKEO_CROSS_SOAK_INTERVAL_SECONDS=10 \
deploy/smoke/cross-language-worker-parity-smoke.sh
```

post-release 追加提交的短跑本地证据：

| 信号 | 结果 |
| --- | ---: |
| 证据目录 | `.dev/reports/cross-language-workers-20260622T065243Z-596956` |
| Soak 轮数 | `2` |
| 派发次数 | `8` 次，覆盖 Go/Rust/Python/Node |
| Succeeded / failed | `8 / 0` |
| Max duration | `2s` |
| Average duration | `2s` |
| Max queue pending | `0` |
| Max outbox pending | `0` |
| Minimum online workers | `7` |
| Verdict | ✅ passed |

证据文件会和 parity report 写在同一目录：`*-soak-summary.json`、`*-soak-summary.csv` 和 `*-soak-metrics.jsonl`；手动 RC workflow 会把它们作为 `cross-language-worker-soak` artifact 上传，并把关键数字写入 GitHub step summary。

## 迁移 CLI 证据

`tikeo-migrate` 作为 review-first 迁移助手已经具备发布条件。Release 包含 Linux、macOS Intel、macOS Apple Silicon 和 Windows 压缩包。预期操作流程仍然是：

1. 在旧项目根目录执行 `tikeo-migrate plan`。
2. 复核 `.tikeo-migration/manifest.json`、`jobs.tikeo.md`、`data-import-plan.json` 和生成的 Java patch 建议。
3. 执行 `tikeo-migrate apply --endpoint <staging> --api-key <key> --dry-run`。
4. 只把复核过的 `ready` jobs 导入预发。
5. 切流前用匹配的 Tikeo Worker 至少触发一个迁移后的作业。

相关文档：[旧调度器迁移指南](../integrations/migrating-from-legacy-schedulers)。

## 通知中心证据边界

通知中心在目标环境中每个启用 provider family 都有真实 test-send 证据后，才可宣称该环境生产就绪。当前实现和文档已经覆盖 channel 行级密钥、provider-specific template、列表/抽屉测试动作、retry/DLQ 证据和脱敏。是否生产就绪仍取决于目标租户/环境里的真实 provider 调用。

相关文档：[通知用户指南](../user-guide/notifications)、[通知中心参考](../reference/notification-center)、[产品就绪验收清单](./product-readiness-acceptance)。

## 下一位 owner 的剩余工作

| 优先级 | 工作 | 停止条件 |
| --- | --- | --- |
| 有云环境时 P0 | 使用外部 DB、ingress/LB/WAF/TLS、NetworkPolicy 和托管数据库 HA 做真实云环境 HA 验收。 | 归档 `scripts/cloud-raft-ha-acceptance.sh` 产物：`summary.json`、`REPORT.md`、cluster diagnostics 和明确 pass/fail 说明。 |
| 下个 release candidate 前 P1 | 通过 `.github/workflows/release-candidate-worker-soak.yml` 手动运行跨语言 soak gate，时长应比短证据更长。 | `TIKEO_CROSS_SOAK_SECONDS=120` 或更长运行产出 `cross-language-worker-soak` artifact，并得到 `failed=0`、`workersOnline` 稳定、`queuePending` 有界、`outboxPending` 不增长。 |
| provider 生产签核前 P1 | 对部署环境实际使用的通知渠道做真实 provider test-send。 | 归档 provider response、message trace、retry/DLQ 状态和脱敏证据。 |
| 大规模迁移推广前 P2 | 用代表性 XXL-JOB 或 PowerJob 旧项目跑 `tikeo-migrate`。 | dry-run apply 加至少一个预发 live trigger，并保留行为对比。 |
| 持续 P2 | 保持公共文档与 release 证据同步。 | docs build、docs contract、search/LLM indexes、README links 和 release asset checks 全部通过。 |
