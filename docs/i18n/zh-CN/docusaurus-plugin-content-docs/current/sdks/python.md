---
title: Python Worker SDK
description: Python SDK 与 Worker demo 的 operator-grade 验收入口。
---

# Python Worker SDK

Python SDK 位于 `sdks/python/tikeo`，可运行 Worker demo 位于 `examples/python/worker-demo`。本文只使用源码可验证事实：配置来自 `src/tikeo/config.py`，Worker Tunnel 来自 `client.py`，Management helper 来自 `management.py`，demo 行为来自 worker-demo 包。Python Worker 是 **outbound-only**：进程主动拨出到 Worker Tunnel，注册 metadata 与 structured capabilities，接收 `DispatchTask`，再回传 task log 和 result；不要把业务 Worker 写成 inbound HTTP Service。

## 依赖坐标

Python 包坐标来自 `sdks/python/tikeo/pyproject.toml`：`name = "tikeo"`、`version = "0.2.0"`、`requires-python = ">=3.11"`。安装发布包时使用不带 `v` 的版本：

```bash
python -m pip install "tikeo==${TIKEO_VERSION}"
```

本仓库 demo 使用 editable 安装：

```bash
cd examples/python/worker-demo
python -m pip install -e ../../../sdks/python/tikeo
python -m pip install -e .
```

SDK 导出 `WorkerConfig`、`local_config`、`Client`、`TaskContext`、`TaskOutcome`、`ManagementClient`、`api_job`、`plugin_api_job`、`script_api_job`、`api_trigger`、`broadcast_api_trigger`、`BroadcastSelectorRequest`、`API_KEY_HEADER` 等符号。

## WorkerConfig 默认值

`local_config(endpoint, client_instance_id)` 返回 `WorkerConfig(endpoint=endpoint, client_instance_id=client_instance_id)`。`WorkerConfig` dataclass 的默认值是：`namespace="default"`，`app="default"`，`name=""` 但 `__post_init__` 会把空 name 改为 `client_instance_id`，`region="local"`，`version="dev"`，`cluster="local"`，`capabilities=[]`，`labels={}`，`structured=WorkerCapabilities()`，`heartbeat_every=timedelta(seconds=10)`。`validate()` 会拒绝空 endpoint、client_instance_id、namespace、app、name、cluster，以及非正 heartbeat。

Python demo 覆盖 operator scope：`TIKEO_WORKER_ENDPOINT` 默认 `http://127.0.0.1:9998`，`TIKEO_WORKER_CLIENT_INSTANCE_ID` 默认 `python-worker-demo-local`，namespace/app 默认 `dev-alpha`/`orders`，cluster/region 默认 `local`，tag 包含 `python` 和 `manual-demo`，默认 processor 为 `demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail,demo.exception`，`worker_pool` 默认 `python-blue`。插件 SQL 默认启用时，会广告 plugin type `sql` 与 processor `billing.sql-sync`。

## 最小 Worker

最小 Python Worker 只需要 config、processor、client 与 outbound session。下面片段保留源码 API，但省略 demo 中的 script runner registry；实际环境只有在工具真实存在时才注册 SRT、Deno、container 或 local command runner。

```python
import os
import time
import tikeo

config = tikeo.local_config(
    os.getenv("TIKEO_WORKER_ENDPOINT", "http://127.0.0.1:9998"),
    os.getenv("TIKEO_WORKER_CLIENT_INSTANCE_ID", "python-worker-demo-local"),
)
config.namespace = "dev-alpha"
config.app = "orders"
config.add_tag("python")
config.add_sdk_processor("demo.echo")

client = tikeo.Client(config)

def process(task: tikeo.TaskContext) -> tikeo.TaskOutcome:
    task.log_info("python echo started")
    return tikeo.succeeded("python demo echo processed")

while True:
    try:
        session = client.connect()
        stop = session.start_heartbeat()
        try:
            session.process_next(process)
        finally:
            stop.set()
            session.close()
    except Exception as exc:
        print(f"worker tunnel ended, reconnecting: {exc}")
        time.sleep(2)
```

能力广告必须保守：`add_sdk_processor`、`add_script_runner`、`add_plugin_processor` 会进入 `structured_capabilities`，调度器会据此选择 Worker。Python 易于调用本地命令，因此更要保证 sandbox runner 来源、PATH、timeout、输出大小、网络权限和 secret refs 都受控。业务任务日志使用 `TaskContext.log_info/log_error`，SDK 诊断日志使用 `configure_logging(LogConfig.from_env())`，两者不要混淆。

## Management API 与管理客户端凭证

Python management helper 位于 `sdks/python/tikeo/src/tikeo/management.py`。`ManagementClient(endpoint, api_key, namespace="default", app="default")` 会 trim endpoint，空 namespace/app 默认 `default`，用 `requests.Session` 注入 `accept: application/json` 和 `x-tikeo-api-key`。凭证必须来自 `TIKEO_MANAGEMENT_API_KEY` 或 Secret store；不要把浏览器 session、OIDC cookie 或人类 bearer token 包装成 SDK key。

helper 名称与语义：`api_job(name, processor_name)` 创建 `scheduleType=api` job，默认 retry policy 为 enabled、3 次、5 秒、2 倍退避、60 秒；`plugin_api_job` 写入 `processorType`；`script_api_job` 写入 `scriptId`；`api_trigger()` 输出 `triggerType=api`、`executionMode=single`；`broadcast_api_trigger(selector)` 输出 `triggerType=api`、`executionMode=broadcast` 和 `broadcastSelector`。

```python
import os
import tikeo

management = tikeo.ManagementClient(
    os.getenv("TIKEO_MANAGEMENT_ENDPOINT", "http://127.0.0.1:9090"),
    os.environ["TIKEO_MANAGEMENT_API_KEY"],
    "dev-alpha",
    "orders",
)
created = management.create_job(tikeo.api_job("python-echo-api", "demo.echo"))
instance = management.trigger_job(created.id, tikeo.api_trigger())
assert instance.trigger_type == "api"
assert instance.execution_mode == "single"

selector = tikeo.BroadcastSelectorRequest(
    tags=["manual-demo"],
    region="us-east-1",
    labels={"worker_pool": "python-blue"},
)
management.trigger_job(created.id, tikeo.broadcast_api_trigger(selector))
```

参考锚点：[`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)、[`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)、[`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)、[`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)、[`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)。

## Demo 行为与脚本边界

Python demo 会按环境变量注册多语言脚本 runner：默认语言集合通常包括 shell、python、javascript、typescript、powershell、php、groovy、rhai；auto sandbox 下 JS/TS 走 Deno，其它语言优先 SRT。`TIKEO_SANDBOX_AUTO_INSTALL` 可关闭工具自动安装。验收时要检查 runner 注册日志和最终 registration，确保只广告真正可执行的 `script_runners`。如果某个 runner 缺失，任务应返回可诊断失败，而不是被静默伪成功。

## 失败与异常 demo

所有语言 demo 都区分业务失败和运行时异常。`demo.fail` 返回正常的 failed `TaskOutcome`，用于验证业务规则失败；`demo.exception` 会 throw、panic、raise 或返回 processor error，用于验证 SDK 能把真实异常栈作为任务日志透传，同时仍把实例结果标记为失败。验收时两个 processor 都要触发：前者证明业务失败语义，后者证明异常堆栈能穿过 Worker Tunnel 并出现在 Notification Center 的执行透传页面。

## 运维依据与排错边界

核对 Python 集成时，先读 `sdks/python/tikeo/src/tikeo/config.py` 中 dataclass 默认值、`validate()` 和 `normalize()`，再读 `client.py` 中 `_register_message`、`Session.start_heartbeat`、`process_next` 和 `TaskResult` 写回。`task.py` 规定 processor 是接收 `TaskContext` 并返回 `TaskOutcome` 的 callable，实例日志只能通过 `log_info` 和 `log_error` 进入任务审计。demo 包把脚本 runner、plugin SQL、dry-run 与 live tunnel 都拆成环境变量，适合现场证明每个开关的影响。排错时先打印或检查 registration，确认 namespace/app、cluster/region、`worker_pool`、`structured.sdk_processors` 与目标 job 匹配，再看 `broadcastSelector` 是否过窄或过宽。Python 能快速调用本地命令，但文档和运维都应坚持最小能力广告，避免把工具缺失误判成业务失败。

## 生产上线检查

上线前把 Python Worker 运行在固定 virtualenv 或不可变容器镜像中，锁定依赖版本，并把 sandbox 工具安装路径纳入变更管理。`client_instance_id` 只用于稳定观测，真正的 Worker 身份、租约、generation 和 fencing token 都来自 Server 注册响应。Python 生态灵活但风险更高：新增本地命令、脚本语言、网络访问或 writable path 前，应先定义审计范围和回滚方式。Management API key 应由 Secret store 注入到 `TIKEO_MANAGEMENT_API_KEY`，只给当前 namespace/app 所需权限；异常处理、SDK debug 日志和任务 payload 日志都不能泄露密钥、secret refs 或敏感输入。

生产观测还应覆盖进程内存、虚拟环境依赖漂移、sandbox 工具解析结果、heartbeat 延迟、任务失败分类和 management 请求错误率。发布后先用单副本连接 live tunnel，再扩容 worker pool，确认能力快照没有因为本机工具差异而变化。

如果 Python Worker 还执行数据处理脚本，建议把输入样本、最大输出、临时目录清理、依赖缓存和网络白名单写入发布检查表，避免同一 processor 在不同主机上表现不一致。

灰度期间保留 dry-run 与单任务运行脚本，便于在不扩大调度范围的情况下复核一个 job instance 的完整生命周期。

灰度完成后再扩大副本数量。

上线复核必须记录版本、配置和命令。

## 现场验收 runbook

1. SDK 测试：`cd sdks/python/tikeo && python -m pip install -e .[test] && python -m pytest`。demo dry-run：`cd examples/python/worker-demo && python -m pip install -e ../../../sdks/python/tikeo && python -m pip install -e . && TIKEO_WORKER_DRY_RUN=1 python -m tikeo_python_worker_demo`。
2. dry-run 期望：输出 registration、structured capabilities、heartbeat sequence；进程不监听业务 HTTP 端口，也不会要求 Server inbound 调用 Worker。
3. live 验收：启动 Server，设置 `TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998`、namespace/app、cluster/region 和 `TIKEO_WORKER_POOL=python-blue`，运行 demo，确认 Web 控制台显示 outbound Worker session。
4. Management 验收：用 `ManagementClient` 创建 `api_job("python-echo-api", "demo.echo")` 并 `api_trigger()`；Server 侧确认 `x-tikeo-api-key` 来源为 `TIKEO_MANAGEMENT_API_KEY`，响应 `triggerType=api` 与 `executionMode=single`。
5. 广播验收：只在需要 fan-out 时使用 `broadcast_api_trigger`，用 `broadcastSelector` 限定 tag `manual-demo` 与 label `worker_pool=python-blue`。
6. 安全验收：禁用或移除 SRT/Deno 后重新启动，确认不可用 runner 不被广告；触发 `demo.fail`，确认失败可见且 Worker 仍保持或重建 tunnel。

## 前置条件

执行本页命令前，请先满足页面列出的安装、认证和权限要求。本地示例默认 Server 使用 `config/dev.toml`，客户端访问 `127.0.0.1`，令牌保存在 shell 变量中，不写入文件或截图。

## 验收

完成本页步骤后，用对应 API、UI、构建、smoke 或部署检查验证结果。有效验收至少包含执行的命令、检查的路由或文件，以及观察到的状态或产物。

## 故障排查

步骤失败时，先保留完整命令、响应状态和 Server 日志时间窗口，再检查认证、namespace/app scope、Worker 匹配、存储 readiness 和代理行为，不要直接修改生产配置。

## 生产检查清单

- [ ] 密钥通过环境变量或平台 Secret 引用管理，不写入示例。
- [ ] 已把本地 `127.0.0.1` 命令替换成真实域名、TLS 和认证方式。
- [ ] 已记录变更面的回滚和证据采集方式。
- [ ] 运维人员可以在没有隐藏 shell 历史或隐式状态的情况下复现验收。
