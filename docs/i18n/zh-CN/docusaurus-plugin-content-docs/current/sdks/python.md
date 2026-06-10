---
title: Python Worker SDK
description: Python SDK 与 Worker demo 的验证入口。
---

# Python Worker SDK

Python SDK 位于 `sdks/python/tikeo`，可运行 Worker demo 位于 `examples/python/worker-demo`。它适合自动化、数据处理 Worker，以及已经标准化 Python runtime 的团队。

## 运行时要求

package 声明 `requires-python = ">=3.11"`，CI 使用 Python 3.12 验证。调整基线时，必须同步 `pyproject.toml`、CI matrix、README 徽章和文档站。

## 从 PyPI 安装

将 `${TIKEO_VERSION}` 替换为 README 顶部 `Python SDK` 徽标显示的版本号。PyPI 使用不带 `v` 的版本字符串。

```bash
python -m pip install "tikeo==${TIKEO_VERSION}"
```

```python
from tikeo import Client, local_config
```

## 验证 SDK

```bash
cd sdks/python/tikeo
python -m pip install -e .[test]
python -m pytest
```

SDK 使用 `grpcio`、`grpcio-tools`、`protobuf` 和 `requests` 支撑 Worker Tunnel 与 management helper surface。

## 验证 demo

```bash
cd examples/python/worker-demo
python -m pip install -e ../../../sdks/python/tikeo
python -m pip install -e .
TIKEO_WORKER_DRY_RUN=1 python -m tikeo_python_worker_demo
```

dry-run mode 可在没有 Server 的情况下验证本地包连接和 capability 声明。

## Live mode 预期

live mode 默认连接 `http://127.0.0.1:9998`，使用 demo 的开发 scope，广告结构化 SDK/plugin/script capability，并对支持的 runner 做 sandbox auto-resolution。运行 live mode 前应先启动 Server，并在 Web 控制台确认 Worker 可见。


## Management API 创建并触发任务

Python management helper 位于 `sdks/python/tikeo/src/tikeo/management.py`。它使用 namespace/app 级 API key header（`x-tikeo-api-key`），密钥应来自 `TIKEO_MANAGEMENT_API_KEY` 这类 Secret；它不是人类登录 session 的包装。`api_job` 创建 `scheduleType=api` 的任务，`api_trigger` 发送 `triggerType=api` 与默认 `executionMode=single`。

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
```

广播需要显式 helper。`broadcast_api_trigger` 会输出 `executionMode=broadcast` 和 `broadcastSelector`；只有所有被选中的 Worker 都应收到本次 API 触发时才使用。

```python
selector = tikeo.BroadcastSelectorRequest(
    tags=["manual-demo"],
    region="us-east-1",
    labels={"worker_pool": "python-blue"},
)
management.trigger_job(created.id, tikeo.broadcast_api_trigger(selector))
```


## Source-backed 参考链接

SDK helper 文档必须锚定到从源码整理出的 API 与协议参考：

- 创建 helper 端点：[`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- 触发 helper 端点：[`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- 实例轮询端点：[`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- 实例日志端点：[`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker 派发消息：[`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## 能力广告纪律

Python Worker 很容易调用本地命令，因此更需要治理边界。只有 runner 已安装并处于受控边界内，才应广告 script capability。任务日志应通过 task context API 输出，SDK diagnostics 应与任务执行证据分离。

## 生产建议

使用 virtualenv 或固定容器镜像保证可重复运行。密钥不要写入代码，只传递 secret reference 或受 scope 限制的环境配置。

## 适合场景

Python Worker 适合数据加工、自动化脚本、内部运维任务和已有 Python 生态的集成。评估时要特别关注依赖隔离、runner 安装来源、超时、输出大小限制和网络访问策略，避免把灵活性变成不可审计的宿主机执行风险。
