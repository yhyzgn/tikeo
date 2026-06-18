---
title: 从 XXL-JOB 或 PowerJob 迁移
sidebar_label: 调度器迁移
description: 针对 XXL-JOB 与 PowerJob 导出的 dry-run 迁移规划。
---

# 从 XXL-JOB 或 PowerJob 迁移

Tikeo 提供一个 report-only 迁移规划工具，帮助团队评估从 XXL-JOB 或 PowerJob 迁移到 Tikeo。它**不会写入 Tikeo 数据库**，只读取 JSON 导出文件，把源任务映射成 Tikeo `create job` 草案，并输出包含 unsupported features 和人工复核项的报告。

迁移前先用它回答三个问题：

1. 哪些源任务可以直接创建成 Tikeo Job？
2. 哪些任务因为 legacy routing、blocking、broadcast、map-reduce 或 worker pinning 语义不完全等价，需要人工复核？
3. Tikeo 中将使用哪些 processor name、schedule、retry policy draft 和 namespace/app？

## 命令

```bash
# JSON report 输出到 stdout
tikeo migrate \
  --from xxl-job \
  --input ./xxl-job-export.json \
  --namespace ops \
  --app billing

# Markdown report 写入文件
tikeo migrate \
  --from powerjob \
  --input ./powerjob-export.json \
  --format markdown \
  --output ./tikeo-migration-report.md
```

`--from` 支持：

| 值 | 来源 |
| --- | --- |
| `xxl-job` | XXL-JOB job export records。 |
| `powerjob` | PowerJob job export records；也兼容 `power-job` alias。 |

支持的 JSON 形态：

- job object 数组；
- `{ "jobs": [...] }`；
- `{ "data": [...] }`；
- `{ "data": { "jobs": [...] } }`；
- `{ "content": [...] }`；
- 单个 job object。

## 输出内容

报告包含：

| 字段 | 含义 |
| --- | --- |
| `source` | `xxl-job` 或 `powerjob`。 |
| `mode` | MVP 阶段固定为 `dry_run_report_only`。 |
| `summary` | total、ready、needs-review、skipped 数量。 |
| `jobs[].tikeoJob` | 包含 namespace、app、name、schedule、processor、enabled、retry policy 和 migration metadata 的草案。 |
| `jobs[].unsupportedFeatures` | 需要人工复核的源调度器特性。 |
| `jobs[].warnings` | 有损映射或缺失字段。 |
| `jobs[].sourceSnapshot` | 保留原始源片段，便于审计/复核。 |

## 映射规则

### XXL-JOB

| 源字段 | Tikeo 草案字段 |
| --- | --- |
| `jobDesc` | `name` |
| `executorAppName` | `app` |
| `executorHandler` | `processorName` |
| `scheduleType=CRON` + `scheduleConf` | `scheduleType=cron`, `scheduleExpr=scheduleConf` |
| `scheduleType=FIX_RATE` | `scheduleType=fixed_rate` |
| `scheduleType=NONE` | `scheduleType=api` |
| `executorFailRetryCount` | `retryPolicy.maxAttempts = retry + 1` |
| `triggerStatus=0` | `enabled=false` |

这些字段会被标记为需要复核，而不是假装完全等价：`glueType`、`executorRouteStrategy`、`executorBlockStrategy`。

### PowerJob

| 源字段 | Tikeo 草案字段 |
| --- | --- |
| `jobName` | `name` |
| `appName` | `app` |
| `processorInfo` | `processorName` |
| `timeExpressionType=2` 或 `CRON` | `scheduleType=cron` |
| `timeExpressionType=3` 或 fixed-rate 名称 | `scheduleType=fixed_rate` |
| `timeExpressionType=4` 或 fixed-delay 名称 | `scheduleType=fixed_delay` |
| `timeExpressionType=1` 或 `API` | `scheduleType=api` |
| `instanceRetryNum` | `retryPolicy.maxAttempts = retry + 1` |
| `status=0` | `enabled=false` |

这些字段会被标记为需要复核：`executeType`、`designatedWorkers`、`maxInstanceNum`。

## 复核流程

1. 把 legacy scheduler jobs 导出为 JSON。
2. 运行 `tikeo migrate`，保存 JSON 或 Markdown 报告。
3. 复核每一个 `needs_review` 项，把旧的 routing/blocking/pinning 语义转换成 Tikeo Worker labels、capabilities、workflow fan-out 或 concurrency policy。
4. 使用 `tikeoJob` 草案手动或通过 Management API 创建小批量 pilot jobs。
5. 启动带匹配 `processorName` 的 Workers。
6. 一次触发一个任务，对比 Tikeo instance logs/results 和旧系统行为，再切流。

## 边界

这个 MVP 是保守的：

- 不连接 XXL-JOB 或 PowerJob 数据库。
- 不自动创建 Tikeo Jobs。
- 不翻译任意 Java executor 代码。
- 不声称 broadcast/map-reduce/blocking/routing 语义完全等价。
- 报告中保留 source snapshots，便于人工审计每个决策。

请把报告当作迁移计划和证据包，而不是一键迁移。
