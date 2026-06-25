---
title: "快速开始：Server + Web + Worker + SDK 触发"
description: 本地启动 Tikeo，初始化 Owner，创建应用级 SDK 凭证，连接 Worker，触发任务并收集验收证据。
---

# 快速开始：Server + Web + Worker + SDK 触发

这个快速开始验证 Tikeo 的真实架构：Server 健康，Web 能运行，应用级 SDK API Key 能创建并触发任务，Worker 通过 Worker Tunnel 主动出站连接，而不是开放入站执行器端口。

## 你会证明什么

完成后应有这些验收证据：Server HTTP API 和 Worker Tunnel listener 正常；当前本地数据库已经创建首个 Owner；namespace/app/worker pool 已创建；service account 和应用级 SDK API Key 已创建；Node.js Worker demo 用 `TIKEO_WORKER_CONNECT=1` 连接并广告 `demo.echo`；SDK Management client 创建 `scheduleType=api` 的 job 并用 `executionMode=single` 触发；实例日志中能看到 `nodejs demo echo processed`。只访问 `/healthz` 不算完成 Worker Tunnel 派发验收。

## 阶段 0：准备本地 shell

```bash
cd tikeo
cargo build --bin tikeo
cd web && bun install --frozen-lockfile && cd ..
cd docs && bun install --frozen-lockfile && cd ..
cd examples/nodejs/worker-demo && bun install --frozen-lockfile && cd ../../..
```

如果你之前运行过 demo，先停止旧 Server/Worker，或者直接使用 smoke 脚本，因为它会创建隔离端口、隔离 DB 和独立证据目录。

## 阶段 1：启动 Server

```bash
cargo run --bin tikeo -- serve --config config/dev.yml
```

`config/dev.yml` 默认 HTTP 是 `0.0.0.0:9090`，Worker Tunnel 是 `0.0.0.0:9998`，存储是 `sqlite://.dev/tikeo-dev.db?mode=rwc`。另一个终端检查：

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

`readyz` 失败时先看 Server 日志，常见原因是 DB 路径权限、端口占用、无效环境变量覆盖或 TLS/plaintext 配置不一致。

## 阶段 2：创建首个 Owner

```bash
curl -fsS http://127.0.0.1:9090/api/v1/auth/bootstrap | jq .
```

如果 `data.registrationOpen` 为 true，注册 Owner，并把注册接口返回的 bearer token 导出给后续步骤：

```bash
BOOTSTRAP_USERNAME="${TIKEO_BOOTSTRAP_USERNAME:-owner-$(date +%s)}"
BOOTSTRAP_EMAIL="${TIKEO_BOOTSTRAP_EMAIL:-${BOOTSTRAP_USERNAME}@example.invalid}"
BOOTSTRAP_PASSWORD="${TIKEO_BOOTSTRAP_PASSWORD:-$(openssl rand -base64 24 | tr -d '\n')}"
BOOTSTRAP_PAYLOAD="$(jq -n \
  --arg username "$BOOTSTRAP_USERNAME" \
  --arg email "$BOOTSTRAP_EMAIL" \
  --arg password "$BOOTSTRAP_PASSWORD" \
  '{username:$username,email:$email,password:$password,confirmPassword:$password}')"
TOKEN="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/bootstrap/register \
  -H 'content-type: application/json' \
  -d "$BOOTSTRAP_PAYLOAD" \
  | tee /tmp/tikeo-bootstrap.json \
  | jq -r .data.token)"
test -n "$TOKEN" && test "$TOKEN" != "null"
printf 'Bootstrap owner: %s\nPassword saved only in this shell variable; store it securely now.\n' "$BOOTSTRAP_USERNAME"
```

如果 bootstrap 已关闭，登录并保存 token：

```bash
: "${TIKEO_BOOTSTRAP_USERNAME:?set the owner username for this DB}"
: "${TIKEO_BOOTSTRAP_PASSWORD:?set the owner password for this DB}"
TOKEN="$(jq -n \
  --arg username "$TIKEO_BOOTSTRAP_USERNAME" \
  --arg password "$TIKEO_BOOTSTRAP_PASSWORD" \
  '{username:$username,password:$password}' \
  | curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/login \
      -H 'content-type: application/json' \
      -d @- | jq -r .data.token)"
test -n "$TOKEN" && test "$TOKEN" != "null"
```

这些凭证只用于本地隔离 DB。共享环境必须使用自己的管理员密码和安全流程。

## 阶段 3：启动 Web 控制台

```bash
cd web
bun run dev -- --host 0.0.0.0 --port 5173 --strictPort
```

打开 `http://127.0.0.1:5173`。Web 适合人工查看 Worker、Jobs、Instances、Audit，但本快速开始继续用 API/脚本记录可重复证据。

## 阶段 4：创建 scope

namespace/app/worker pool 必须和 Worker、SDK Management client 一致：

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/namespaces \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"name":"sdk-smoke"}' | jq .

curl -fsS -X POST http://127.0.0.1:9090/api/v1/apps \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"namespace":"sdk-smoke","name":"management"}' | jq .

curl -fsS -X POST http://127.0.0.1:9090/api/v1/worker-pools \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"namespace":"sdk-smoke","app":"management","name":"nodejs-blue"}' | jq .
```

如果 job 和 Worker 不在同一个 namespace/app，即使 `processorName` 正确也不会匹配。

## 阶段 5：创建应用级 SDK API Key

SDK Management client 使用 `x-tikeo-api-key`，不是人类登录 bearer token。先创建 service account：

```bash
SERVICE_ACCOUNT_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/management/service-accounts \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"name":"quickstart-sa","description":"Quickstart machine identity","namespace":"sdk-smoke","app":"management","workerPool":"nodejs-blue"}' | jq -r .data.id)"
```

再创建 API Key：

```bash
TIKEO_MANAGEMENT_API_KEY="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/management/api-keys \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d "{\"name\":\"quickstart-management-key\",\"namespace\":\"sdk-smoke\",\"app\":\"management\",\"service_account_id\":\"$SERVICE_ACCOUNT_ID\",\"scopes\":[\"jobs:read\",\"jobs:write\",\"instances:execute\"],\"expires_at\":null}" | jq -r .data.api_key)"
export TIKEO_MANAGEMENT_API_KEY
```

验证：

```bash
curl -fsS http://127.0.0.1:9090/api/v1/jobs -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY" | jq .code
```

## 阶段 6：Worker 主动出站连接

```bash
cd examples/nodejs/worker-demo
TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998 \
TIKEO_WORKER_CONNECT=1 \
TIKEO_WORKER_NAMESPACE=sdk-smoke \
TIKEO_WORKER_APP=management \
TIKEO_WORKER_POOL=nodejs-blue \
TIKEO_WORKER_CLUSTER=local \
TIKEO_WORKER_REGION=local \
TIKEO_WORKER_CLIENT_INSTANCE_ID=nodejs-quickstart-worker \
TIKEO_WORKER_NORMAL_PROCESSORS=demo.echo \
TIKEO_ENABLE_PLUGIN_SQL=0 \
TIKEO_SANDBOX_AUTO_INSTALL=0 \
bun start
```

预期日志包括 `nodejs worker demo configured`、`nodejs worker connected` 和包含 `demo.echo` 的 structured capability。业务 Worker 不需要入站 Service 或 HTTP 回调端口。

## 阶段 7：用 SDK 创建并触发任务

从仓库根目录的命令终端创建临时 Bun 脚本，这样相对源码 import 才能解析：

```bash
cat >tikeo-quickstart-trigger.ts <<'TS'
import { ManagementClient, apiJob, apiTrigger } from "./sdks/nodejs/tikeo/src/index";
const management = new ManagementClient(
  process.env.TIKEO_MANAGEMENT_ENDPOINT ?? "http://127.0.0.1:9090",
  process.env.TIKEO_MANAGEMENT_API_KEY ?? "",
  "sdk-smoke",
  "management",
);
const created = await management.createJob(apiJob("quickstart-nodejs-echo", "demo.echo"));
const instance = await management.triggerJob(created.id, apiTrigger());
console.log(JSON.stringify({ jobId: created.id, instanceId: instance.id, triggerType: instance.triggerType, executionMode: instance.executionMode }, null, 2));
TS

TIKEO_MANAGEMENT_ENDPOINT=http://127.0.0.1:9090 \
TIKEO_MANAGEMENT_API_KEY="$TIKEO_MANAGEMENT_API_KEY" \
bun tikeo-quickstart-trigger.ts
rm -f tikeo-quickstart-trigger.ts
```

如果有意从仓库外运行，先安装 `@yhyzgn/tikeo`，并把 import 改成已安装的包名；上面的仓库源码 import 只保证在仓库根目录可运行。

## 验收证据

```bash
curl -fsS http://127.0.0.1:9090/api/v1/workers \
  -H "authorization: Bearer $TOKEN" | jq '.data.items[] | {clientInstanceId,status,namespace,app,structuredCapabilities}'
```

再从 Web 或 `/api/v1/instances` 检查实例与日志。Node.js 成功路径应出现 `nodejs demo echo processed`。

## 自动化验收路径

推荐直接运行维护中的脚本：

```bash
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

它会在 `.dev/reports/management-trigger-e2e-*` 下保存生成的 TOML、SQLite DB、Server log、Worker log、service-account JSON、API-key JSON、case JSONL、summary JSON 和最终 report。没有这些证据，不要宣称 SDK create+trigger 链路完成。

## 清理和排障

停止 Worker/Server 用 `Ctrl-C`。`.dev/tikeo-dev.db` 是 Git 忽略的本地运行态数据库，普通重启、拉取代码或切分支不应该替换它；只有确实要重置本地 bootstrap 状态时才删除 `.dev/tikeo-dev.db .dev/tikeo-dev.db-shm .dev/tikeo-dev.db-wal`。常见问题：`readyz` 失败看 DB/端口/配置；bootstrap closed 说明 DB 已有 Owner；SDK key unauthorized 说明 scopes 或 scope 不对；Worker online 但任务 pending 说明 namespace/app/processor 不匹配；Worker 不出现通常是 endpoint、TLS 或 dry-run 模式问题。

## 为什么这些步骤不能省略

Tikeo 的价值不在于 Server 单独能启动，而在于 Server、scope、SDK 凭证、Worker Tunnel、processor capability、实例证据之间形成闭环。很多传统调度器的问题都出现在“调度中心以为任务已经派发，但执行侧不可达或不可解释”。因此快速开始必须同时验证控制面和执行面：如果只创建 job，没有 Worker，就只能证明管理 API；如果只启动 Worker，没有 SDK create+trigger，就不能证明应用接入；如果只看 Web 页面，没有实例日志，就不能证明执行证据。

每个 scope 字段都应当被当作调度条件。`namespace` 表示租户/环境边界，`app` 表示应用边界，`worker_pool` 用作运营标签和容量分组，`processorName` 对应 SDK 代码中真实注册的处理器。排障时不要凭感觉说“Worker 已经在线所以应该执行”，而是检查这些字段是否在 job、Worker、API Key 和 trigger 中一致。

## 何时进入下一阶段

只有在看到 online Worker、API-triggered instance、`executionMode=single`、Worker task log 和成功结果之后，才进入部署或 SDK 集成下一阶段。如果当前环境缺少 Docker、Helm 或某个语言工具，不要缩小验收标准；应该记录具体缺口，并使用能运行的 smoke 路径完成同等证据。这个项目处于功能/模块验收阶段，文档必须引导读者补齐真实链路，而不是把失败解释成“以后再说”。

## 前置条件

执行本页命令前，请先满足页面列出的安装、认证和权限要求。本地示例默认 Server 使用 `config/dev.yml`，客户端访问 `127.0.0.1`，令牌保存在 shell 变量中，不写入文件或截图。

## 故障排查

步骤失败时，先保留完整命令、响应状态和 Server 日志时间窗口，再检查认证、namespace/app scope、Worker 匹配、存储 readiness 和代理行为，不要直接修改生产配置。

## 生产检查清单

- [ ] 密钥通过环境变量或平台 Secret 引用管理，不写入示例。
- [ ] 已把本地 `127.0.0.1` 命令替换成真实域名、TLS 和认证方式。
- [ ] 已记录变更面的回滚和证据采集方式。
- [ ] 运维人员可以在没有隐藏 shell 历史或隐式状态的情况下复现验收。
