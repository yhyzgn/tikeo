---
title: 从 XXL-JOB 或 PowerJob 迁移
sidebar_label: 调度器迁移
description: 针对 XXL-JOB 与 PowerJob 导出的 dry-run 迁移规划。
---

# 从 XXL-JOB 或 PowerJob 迁移

Tikeo 提供独立的 `tikeo-migrate` CLI，帮助团队从 XXL-JOB 或 PowerJob 迁移到 Tikeo。默认的 `plan` 命令是非破坏性的：读取 JSON 导出文件，把源任务映射成 Tikeo `create job` 草案，可选扫描 Java/Spring Worker 项目，并生成包含报告、Java 依赖建议、处理器注解补丁、unsupported features 和人工复核项的迁移包。

迁移前先用它回答三个问题：

1. 哪些源任务可以直接创建成 Tikeo Job？
2. 哪些任务因为 legacy routing、blocking、broadcast、map-reduce 或 worker pinning 语义不完全等价，需要人工复核？
3. Tikeo 中将使用哪些 processor name、schedule、retry policy draft 和 namespace/app？

## 命令

### 推荐的约定优先流程

把旧调度器导出的 JSON 放在旧 Worker 项目根目录，并在该目录执行工具。这个布局下，迁移规划不需要手动指定发现类参数：

```bash
cd ./legacy-worker

# 在 ./.tikeo-migration 中生成完整、非破坏性的迁移包
tikeo-migrate plan

# 复核迁移包后，先 dry-run API 写入。
# apply-data 的 --bundle 也默认读取 ./.tikeo-migration。
tikeo-migrate apply-data \
  --endpoint http://127.0.0.1:9090 \
  --api-key "$TIKEO_MIGRATION_API_KEY" \
  --dry-run
```

自动探测规则：

| 输入 | 约定 |
| --- | --- |
| 项目根目录 | 当前目录包含 `pom.xml`、`build.gradle` 或 `build.gradle.kts` 时，自动作为 Java 项目根目录。 |
| 导出文件 | 明确命名的单个 JSON 文件，例如 `xxl-job-export.json`、`xxljob-export.json`、`powerjob-export.json`、`power-job-export.json`、`jobs-export.json`，或 `export/`、`exports/`、`migration/` 下匹配的 JSON 文件。 |
| 来源调度器 | 优先根据文件名判断，其次根据 JSON 内容判断，例如 XXL-JOB 的 `executorHandler`/`jobDesc`/`scheduleConf`，或 PowerJob 的 `processorInfo`/`timeExpressionType`/`instanceRetryNum`。 |
| 迁移包输出 | `./.tikeo-migration`。 |

如果发现多个可能的导出文件，或者无法安全推断来源，命令会明确报错并要求传覆盖参数，而不是随便猜。

### 非标准目录的覆盖参数

```bash
tikeo-migrate plan \
  --from xxl-job \
  --input ./exports/jobs.json \
  --project ./legacy-worker \
  --output-dir ./migration-bundle \
  --namespace ops \
  --app billing

tikeo-migrate apply-data \
  --bundle ./migration-bundle \
  --endpoint http://127.0.0.1:9090 \
  --api-key "$TIKEO_MIGRATION_API_KEY" \
  --dry-run
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

迁移包包含：

| 字段 | 含义 |
| --- | --- |
| `manifest.json` | 包含数据、代码和 checklist 的完整迁移包 manifest。 |
| `jobs.tikeo.json` / `jobs.tikeo.md` | Job 迁移报告，包含 total、ready、needs-review、skipped。 |
| `data-import-plan.json` | 分离 ready 与 needs-review 的 Tikeo Job 草案，便于受控写入。 |
| `java-project-plan.json` / `.md` | 检测到的 build system、Spring Boot major、推荐 Tikeo artifact、handler candidates 和 review notes。 |
| `java-patches/*.patch` | review-first 的依赖和 handler 注解补丁建议。 |
| `CHECKLIST.md` | 分支复核、staging 导入、单任务触发和双跑对账的人工验收流程。 |

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

1. 把 legacy scheduler jobs 导出为 JSON；可以的话直接放到旧 Worker 项目根目录。
2. 在该项目根目录执行 `tikeo-migrate plan`。只有非标准目录结构才使用 `--input`、`--from`、`--project` 或 `--output-dir` 覆盖。
3. 复核每一个 `needs_review` 项，把旧的 routing/blocking/pinning 语义转换成 Tikeo Worker labels、capabilities、workflow fan-out 或 concurrency policy。
4. 在分支上应用生成的 Java patches，补充推荐 starter 依赖，并人工适配复杂 handler 签名。
5. 先运行 `tikeo-migrate apply-data --dry-run`，再在 staging 去掉 `--dry-run` 写入 ready jobs。
6. 启动带匹配 `processorName` 的 Workers。
7. 一次触发一个任务，对比 Tikeo instance logs/results 和旧系统行为，再切流。

## 边界

这个 MVP 是保守的：

- `plan` 不连接 XXL-JOB 或 PowerJob 数据库。
- `plan` 不自动创建 Tikeo Jobs，也不直接修改旧项目源码。
- `apply-data` 是唯一会调用 Tikeo Management API 的命令，并支持 `--dry-run`。
- Java patches 只覆盖依赖插入和 handler 注解建议；任意 executor/业务代码仍需人工复核。
- 不声称 broadcast/map-reduce/blocking/routing 语义完全等价。
- 报告中保留 source snapshots，便于人工审计每个决策。

请把迁移包当作受控迁移计划和证据包，而不是盲目一键迁移。
