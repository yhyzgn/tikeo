---
title: 日志配置
description: Tikeo Server 日志配置完整参考，包含 root filter、HTTP 明细日志、SQL 日志、console/file/ELK 渠道、默认值、性能影响和安全注意事项。
---

# 日志配置

Tikeo Server 的日志配置位于 `config/tikeo.yml` 或 `config/dev.yml` 的 `observability.logging` 下。相同配置项也可以通过 `TIKEO__...` 环境变量覆盖，但生产环境应优先把挂载的 YAML 文件作为配置来源。

日志模型分为四层：

1. `root`：未设置 `RUST_LOG` 时的默认应用日志过滤器。
2. `http`：HTTP 访问摘要，以及可选的请求/响应明细采集。
3. `sql`：SQL driver/ORM 日志。
4. `channels`：输出渠道。未启用的渠道不会被构造，因此不会加载对应 writer、formatter 或远程转发线程。

## 完整示例

```yaml
observability:
  logging:
    root:
      level: INFO

    http:
      level: INFO
      include_headers: false
      include_body: false
      max_body_bytes: 65536

    sql:
      enabled: false
      level: DEBUG
      include_values: false
      slow_threshold_ms: 250

    channels:
      console:
        enabled: true
        level: INFO

      file:
        enabled: false
        level: INFO
        path: "${TIKEO_LOG_PATH:/logs}"

      elk:
        enabled: ${ELK_ENABLED:false}
        servers: "${ELK_SERVERS:elk-server:8094}"
        topic: "${ELK_TOPIC:ivs-dev}"
        level: INFO
        sasl:
          enabled: ${ELK_SASL_ENABLED:false}
          username: "${ELK_USERNAME:}"
          password: "${ELK_PASSWORD:}"
```

`config/dev.yml` 使用同样的结构，但本地开发默认把 console 渠道设为 `DEBUG`，文件日志路径默认设为 `.dev/logs`。

## 级别与路由模型

Tikeo 内部使用 `tracing` target。实际生效的过滤规则由以下层共同决定：

| 层级 | 控制范围 | 说明 |
| --- | --- | --- |
| `root.level` | Tikeo 应用 target 和部分 runtime target 的默认启用级别。 | 仅在未设置 `RUST_LOG` 时使用。非法值回退到 `INFO`。 |
| `http.level` | 明细日志 target：`tikeo_server::http::detail`。 | 不改变 HTTP 摘要日志等级，只控制可选的请求/响应明细记录。 |
| `sql.enabled` 与 `sql.level` | `sqlx`，以及启用值日志时的 `sea_orm` SQL target。 | `sql.enabled=false` 时，即使 root level 很宽，SQL target 也会被关闭。 |
| `channels.*.level` | 每个输出 sink 接收的最低等级。 | sink 可以比 root/detail 更严格，但无法输出上游已过滤掉的事件。 |
| `RUST_LOG` | 紧急进程级覆盖。 | 优先使用 YAML 配置；设置后 Tikeo 仍会追加 SQL 指令，因此关闭的 SQL 日志不会被误打开。 |

可用等级：`TRACE`、`DEBUG`、`INFO`、`WARN`/`WARNING`、`ERROR`。

## `root`

| 配置项 | 默认值 | 环境变量 | 影响 |
| --- | --- | --- | --- |
| `observability.logging.root.level` | `INFO` | `TIKEO__OBSERVABILITY__LOGGING__ROOT__LEVEL` | Server、storage、config、部分 HTTP/runtime 库和普通应用事件的默认日志级别。 |

运维注意事项：

- 生产环境默认保持 `INFO`。
- `DEBUG` 和 `TRACE` 会增加事件量和字符串格式化成本。
- 设置 `RUST_LOG` 后会成为主过滤器；建议只用于紧急覆盖或一次性本地调试。

## HTTP 日志

HTTP 日志分为**摘要日志**和**明细日志**。

摘要日志始终按结果输出：

| 结果 | 等级 | 消息 |
| --- | --- | --- |
| 收到请求 | `INFO` | `HTTP request received` |
| 成功或非错误响应 | `INFO` | `HTTP request completed` |
| 4xx 响应 | `WARN` | `HTTP request completed with client error` |
| 5xx 响应 | `ERROR` | `HTTP request completed with server error` |

摘要记录包含 `trace_id`、method、path、query、status、接口耗时，以及可用时的请求/响应大小。响应日志中的 latency 是接口端到端耗时。

明细日志只有在 `include_headers` 或 `include_body` 开启，并且 `tikeo_server::http::detail` target 在 `http.level` 下启用时才输出。

| 配置项 | 默认值 | 环境变量 | 影响 | 注意事项 |
| --- | --- | --- | --- | --- |
| `observability.logging.http.level` | `INFO` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__LEVEL` | `HTTP request detail` 与 `HTTP response detail` 事件等级。 | 不会关闭摘要日志。 |
| `observability.logging.http.include_headers` | `false` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__INCLUDE_HEADERS` | 在明细日志中采集请求/响应 header。 | `authorization`、`cookie`、`set-cookie`、`x-api-key` 等内置敏感 header 会脱敏，但自定义敏感 header 仍需要上游治理。 |
| `observability.logging.http.include_body` | `false` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__INCLUDE_BODY` | 在明细日志中采集请求/响应 body。 | 可能暴露凭据、token、业务 payload、脚本和业务内容。只能在短时间调试窗口开启。 |
| `observability.logging.http.max_body_bytes` | `65536` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__MAX_BODY_BYTES` | 每个请求或响应 body 最多采集的字节数。 | 值越大，启用 body 日志的请求内存复制和延迟成本越高。 |

以下流式或二进制类 body 会被跳过：

- `text/event-stream` SSE 响应
- `application/grpc`
- `multipart/*`
- `application/octet-stream`
- 无 content length 的 chunked body
- 在配置采集上限内读取失败的 body

推荐用法：

```yaml
# 仅用于短时间本地调试窗口。
observability:
  logging:
    http:
      level: DEBUG
      include_headers: true
      include_body: true
      max_body_bytes: 16384
```

故障处理结束后把 `include_body` 改回 `false`。生产环境默认不要打开 header/body 明细，除非有受控的短时间调试需要。

## SQL 日志

SQL 日志默认关闭，因为高频数据库日志会带来噪声、成本和敏感数据风险。

| 配置项 | 默认值 | 环境变量 | 影响 | 注意事项 |
| --- | --- | --- | --- | --- |
| `observability.logging.sql.enabled` | `false` | `TIKEO__OBSERVABILITY__LOGGING__SQL__ENABLED` | 启用 SQL driver/ORM 执行日志。 | 常规生产运行保持关闭。 |
| `observability.logging.sql.level` | `DEBUG` | `TIKEO__OBSERVABILITY__LOGGING__SQL__LEVEL` | 已启用 SQL 事件的日志等级。 | 排障建议用 `DEBUG`；`TRACE` 可能非常嘈杂。 |
| `observability.logging.sql.include_values` | `false` | `TIKEO__OBSERVABILITY__LOGGING__SQL__INCLUDE_VALUES` | 在 driver/ORM 支持时输出绑定参数值。 | 可能暴露凭据、token、scope 名称、payload 和业务数据。只能在最短窗口开启。 |
| `observability.logging.sql.slow_threshold_ms` | `250` | `TIKEO__OBSERVABILITY__LOGGING__SQL__SLOW_THRESHOLD_MS` | storage driver options 使用的慢语句阈值。 | 阈值越低，WARN 数量越多。 |

推荐生产排障配置：

```yaml
observability:
  logging:
    sql:
      enabled: true
      level: DEBUG
      include_values: false
      slow_threshold_ms: 250
```

只有在已批准、限时且日志落点安全的情况下，才开启 `include_values=true`。

## 输出渠道

### Console 渠道

| 配置项 | 默认值 | 环境变量 | 影响 |
| --- | --- | --- | --- |
| `observability.logging.channels.console.enabled` | `true` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__CONSOLE__ENABLED` | 启用 stdout/stderr console 输出。 |
| `observability.logging.channels.console.level` | `INFO` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__CONSOLE__LEVEL` | 写入 console 的最低等级。 |

Console 输出是容器和 Kubernetes 的推荐路径。启用时支持 ANSI 高亮，并通过 `[HTTP]`、`[SQL ]`、`[APP ]` 标签区分事件类别。

影响：

- 适合作为容器默认方案，由平台日志采集器负责持久化和轮转。
- `DEBUG` 或 `TRACE` 仍可能产生大量 stdout。
- 关闭后不会安装 console formatter/writer layer。

### File 渠道

| 配置项 | 默认值 | 环境变量 | 影响 |
| --- | --- | --- | --- |
| `observability.logging.channels.file.enabled` | `false` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__ENABLED` | 启用 JSON 文件日志 sink。 |
| `observability.logging.channels.file.level` | `INFO` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__LEVEL` | 写入文件 sink 的最低等级。 |
| `observability.logging.channels.file.path` | `/logs` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__PATH` 或模板变量 `TIKEO_LOG_PATH` | 文件日志目录或文件路径。目录路径写入 `tikeo.log`；文件路径使用指定文件名。 |

文件日志是 JSON Lines，字段包含 `timestamp`、`level`、`target`、`message` 和可选 `fields`。

影响和运维要求：

- sink 使用非阻塞 writer，请求路径只入队格式化后的事件，不直接做阻塞文件 I/O。
- 只有存在持久日志卷时才启用。容器中挂载 `/logs`，或把 `TIKEO_LOG_PATH` 指向挂载目录。
- 轮转和保留策略由运维负责，例如平台日志 agent、logrotate 或 sidecar 策略。
- 关闭后 Tikeo 不创建目录，也不安装 file writer。

### ELK / 远程采集渠道

| 配置项 | 默认值 | 环境变量 | 影响 |
| --- | --- | --- | --- |
| `observability.logging.channels.elk.enabled` | `false` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__ENABLED` 或模板变量 `ELK_ENABLED` | 启用远程 JSON-lines 转发。 |
| `observability.logging.channels.elk.servers` | `config/tikeo.yml` 中为 `elk-server:8094` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SERVERS` 或 `ELK_SERVERS` | 逗号分隔的 `host:port` 采集器列表。 |
| `observability.logging.channels.elk.topic` | `ivs-dev` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__TOPIC` 或 `ELK_TOPIC` | 逻辑 topic/index 元数据。 |
| `observability.logging.channels.elk.level` | `INFO` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__LEVEL` | 远程转发的最低等级。 |
| `observability.logging.channels.elk.sasl.enabled` | `false` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__ENABLED` 或 `ELK_SASL_ENABLED` | 兼容采集环境的 SASL 元数据开关。 |
| `observability.logging.channels.elk.sasl.username` | 空 | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__USERNAME` 或 `ELK_USERNAME` | 可选 SASL username 元数据。 |
| `observability.logging.channels.elk.sasl.password` | 空 | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__PASSWORD` 或 `ELK_PASSWORD` | 可选 SASL password 元数据。应通过 Secret 环境变量注入，不要提交到 Git。 |

ELK 输出使用适配日志采集器的扁平 JSON Lines：

```json
{
  "app": "tikeo-server",
  "ip": null,
  "hostname": "tikeo-0",
  "class": "tikeo_server::http::trace",
  "file": "crates/tikeo-server/src/http/trace.rs",
  "method": "tikeo_server::http::trace",
  "line": "150",
  "datetime": "2026-06-26T00:00:00Z",
  "thread": "tokio-runtime-worker",
  "level": "INFO",
  "trace_id": "trc-...",
  "msg": "HTTP request completed | method=GET | path=/readyz | status=200 | latency_ms=1.2",
  "exception": ""
}
```

影响和运维要求：

- 转发是非阻塞、批量发送。应用线程入队日志帧，由专用 `tikeo-elk-log-forwarder` 线程刷到采集器。
- 有界队列满或连接断开时，日志帧可能被丢弃，而不是阻塞业务流量。
- `servers` 可以配置多个采集器以便故障切换。
- `sasl.*` 是兼容采集环境的配置元数据，不是网络 TLS、防火墙或采集端访问控制的替代品。
- 关闭后不会安装远程 formatter，也不会启动远程转发线程。

## 环境变量速查

| YAML 配置项 | 主环境变量覆盖 | 模板快捷变量 |
| --- | --- | --- |
| `observability.logging.root.level` | `TIKEO__OBSERVABILITY__LOGGING__ROOT__LEVEL` | — |
| `observability.logging.http.level` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__LEVEL` | — |
| `observability.logging.http.include_headers` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__INCLUDE_HEADERS` | — |
| `observability.logging.http.include_body` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__INCLUDE_BODY` | — |
| `observability.logging.http.max_body_bytes` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__MAX_BODY_BYTES` | — |
| `observability.logging.sql.enabled` | `TIKEO__OBSERVABILITY__LOGGING__SQL__ENABLED` | — |
| `observability.logging.sql.level` | `TIKEO__OBSERVABILITY__LOGGING__SQL__LEVEL` | — |
| `observability.logging.sql.include_values` | `TIKEO__OBSERVABILITY__LOGGING__SQL__INCLUDE_VALUES` | — |
| `observability.logging.sql.slow_threshold_ms` | `TIKEO__OBSERVABILITY__LOGGING__SQL__SLOW_THRESHOLD_MS` | — |
| `observability.logging.channels.console.enabled` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__CONSOLE__ENABLED` | — |
| `observability.logging.channels.console.level` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__CONSOLE__LEVEL` | — |
| `observability.logging.channels.file.enabled` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__ENABLED` | — |
| `observability.logging.channels.file.level` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__LEVEL` | — |
| `observability.logging.channels.file.path` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__PATH` | 提交模板中的 `TIKEO_LOG_PATH` |
| `observability.logging.channels.elk.enabled` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__ENABLED` | 提交模板中的 `ELK_ENABLED` |
| `observability.logging.channels.elk.servers` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SERVERS` | 提交模板中的 `ELK_SERVERS` |
| `observability.logging.channels.elk.topic` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__TOPIC` | 提交模板中的 `ELK_TOPIC` |
| `observability.logging.channels.elk.level` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__LEVEL` | — |
| `observability.logging.channels.elk.sasl.enabled` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__ENABLED` | 提交模板中的 `ELK_SASL_ENABLED` |
| `observability.logging.channels.elk.sasl.username` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__USERNAME` | 提交模板中的 `ELK_USERNAME` |
| `observability.logging.channels.elk.sasl.password` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__PASSWORD` | 提交模板中的 `ELK_PASSWORD` |

## 推荐配置画像

### 本地开发

```yaml
observability:
  logging:
    root:
      level: DEBUG
    channels:
      console:
        enabled: true
        level: DEBUG
      file:
        enabled: false
      elk:
        enabled: false
```

用于本地迭代。只有在特定调试窗口内再叠加 `http.include_body=true` 或 `sql.enabled=true`。

### 生产容器默认配置

```yaml
observability:
  logging:
    root:
      level: INFO
    http:
      level: INFO
      include_headers: false
      include_body: false
      max_body_bytes: 65536
    sql:
      enabled: false
      level: DEBUG
      include_values: false
      slow_threshold_ms: 250
    channels:
      console:
        enabled: true
        level: INFO
      file:
        enabled: false
      elk:
        enabled: false
```

使用 console 输出，由 Kubernetes、Docker 或主机采集器负责持久化。

### 生产启用文件日志

```yaml
observability:
  logging:
    channels:
      console:
        enabled: true
        level: INFO
      file:
        enabled: true
        level: INFO
        path: /logs
```

需要把 `/logs` 挂载为持久可写卷，并配置外部轮转。

### 生产启用远程采集

```yaml
observability:
  logging:
    channels:
      console:
        enabled: true
        level: INFO
      elk:
        enabled: true
        servers: "elk-a:8094,elk-b:8094"
        topic: "tikeo-prod"
        level: INFO
        sasl:
          enabled: false
```

除非故障明确要求，否则保持 HTTP body 日志和 SQL 值日志关闭。

## 运维注意事项

- 生产环境不要长期打开大范围 `TRACE` 日志，除非故障需要且窗口很短。
- 不要把完整 HTTP body 日志或 SQL 参数值日志作为常规配置。两者都可能暴露敏感业务数据。
- 启用文件日志前先确认有持久挂载，否则容器重启后日志可能丢失。
- 容器平台优先使用 console 日志，并由平台日志 agent 采集 stdout。
- 未启用的输出渠道不会加载，这是未使用 sink 的最低开销模式。
- `RUST_LOG` 只用于紧急覆盖；长期生效的行为应写入 `observability.logging`。
