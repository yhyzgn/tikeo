# Worker 身份、会话生命周期与失联判定设计方案

## 1. 背景与问题

当前 tikee Worker Tunnel 的核心约束是：Worker 主动出站连接 Server，`worker_id` 由 Server 在注册成功后下发，客户端只能上报 `client_instance_id` 作为提示。这符合 K8s/Docker 模型，也同样适用于裸机、VM、systemd、Supervisor、Windows Service 等常规服务器部署：每次进程启动都可以视为一次新的运行会话，但这些环境往往缺少 Pod UID 这类天然实例标识，因此更需要明确的 logical identity 策略。

用户提出的关键问题是：如果 demo 或生产服务每启动一次都注册一个新的 Worker，系统如何区分“异常失联 / 网络掉线 / 正常迭代后成为历史”的 Worker？如果不区分，Worker 列表、调度、排障和指标都会逐渐混乱。

本设计的核心判断：

- **每次连接生成新的 `worker_id` 是正确且安全的**，因为它代表一次具体 Worker Tunnel 会话/运行 incarnation，避免旧连接、僵尸连接和新进程复用同一权威 ID 导致状态串线。
- **仅靠 `worker_id` 不足以表达生产语义**，必须引入稳定逻辑身份、会话代际、租约状态、替换原因和历史保留策略。
- **失联原因无法在第一时间 100% 精确判定**。系统只能根据收到的信号做“证据分级”：graceful close 可以明确判定正常下线；同一 logical instance 新连接可以判定 replaced；仅心跳超时只能判定 `lease_expired_unknown`，后续通过 K8s 事件、主机/systemd 事件、重连、用户标记或 TTL 归档进一步归类。

## 2. 设计目标

1. 保留 Server 分配 ephemeral `worker_id` 的安全模型。
2. 同时支持 K8s/Docker 和裸机/VM/systemd 等常规服务器部署中的频繁重启、滚动更新、扩缩容或进程守护重拉起，不污染在线 Worker 视图。
3. 区分稳定逻辑 Worker 与一次具体连接会话。
4. 将失联判定从单一 offline 升级为带原因和证据等级的状态机。
5. 对同一逻辑实例的重连做 fencing，避免旧连接继续写入心跳、日志或结果污染新会话。
6. 为 Web Worker 集群页面提供“在线 / 异常 / 历史”分层视图。
7. 为调度器提供明确可用性判断：只调度到当前 online、未 drain、租约有效、代际最新的 Worker session。

## 3. 概念模型

### 3.1 Worker Pool

Worker Pool 是稳定的调度与治理边界，属于 `namespace + app`，可配置能力、标签选择器、并发、quota、脚本权限和隔离策略。它不表示某个进程。

### 3.2 Logical Worker Instance

Logical Worker Instance 表示客户端声明的稳定运行单元，用于把多次会话串起来。建议唯一键：

```text
namespace + app + cluster + region + client_instance_id
```

`client_instance_id` 不是权威执行身份，而是逻辑归并 hint。生产推荐值：

1. K8s StatefulSet：`$(POD_NAME)` 或 `$(STATEFULSET_ORDINAL)`。
2. K8s Deployment：优先 `$(POD_UID)`；如果希望重启后聚合到同一 Pod 名，则用 `$(POD_NAME)`。
3. Docker/Compose：容器名、container id 或 hostname。
4. 裸机/VM/systemd：推荐显式配置 `service_name + host_id + instance_slot`，例如 `billing-worker@host-a#slot-1`。`host_id` 可来自 `/etc/machine-id`、云厂商 instance id、CMDB asset id 或运维配置的稳定主机名。
5. 多进程同机部署：必须加入 `instance_slot` / `process_group` / systemd template instance（如 `%i`），避免同一主机多个 Worker 进程使用同一个 logical key。
6. 本地开发：SDK 自动生成 `hostname + process_start_id`，避免多个本地进程冲突。

### 3.3 Worker Session / Incarnation

Worker Session 表示一次实际 Worker Tunnel 连接。每次注册成功生成新的 Server authoritative `worker_id`，并分配：

- `connection_id`：当前 Server 内存连接路由 ID。
- `generation`：同一 Logical Worker 的单调递增代际。
- `fencing_token`：Server 下发并用于后续心跳、日志、结果校验的会话 fencing 值。
- `lease_expires_at`：心跳租约到期时间。

因此：

```text
Logical Worker Instance 1 --- has many ---> Worker Session / worker_id
```


### 3.4 裸机/VM 部署身份规则

裸机和 VM 环境没有 Pod UID，也不一定有稳定容器 ID。tikee 不应假设 Worker 一定运行在容器内。常规服务器部署应遵循：

- `host_id` 表示机器或 VM 的稳定身份，优先级：显式 `TIKEE_WORKER_HOST_ID` > 云 instance id > `/etc/machine-id` > 稳定 hostname。
- `instance_slot` 表示同一主机上的第几个 Worker 实例，适配 systemd template、Supervisor program name、Windows Service name 或手工多进程部署。
- `client_instance_id` 可以由 SDK 组合为：`{service_name}@{host_id}#{instance_slot}`。
- 如果业务希望“每次进程启动都作为新 logical instance”，可以显式设置 `identity_mode=ephemeral`；否则默认推荐 `stable_host_slot`。
- 裸机滚动发布通常不会产生新 host id，因此旧 session 被新 generation 替换是预期行为；但如果同一 host/slot 长时间 heartbeat timeout 且没有新 generation，应显示为“租约过期，原因未确认”，不能直接归档为历史。

## 4. 状态机与原因码

### 4.1 Logical Worker 状态

Logical Worker 是聚合状态，用于 UI 和排障：

- `active`：存在最新 online session。
- `degraded`：最新 session suspect/offline，但仍在短保留窗口内。
- `replaced`：最新状态来自新 session 替换旧 session。
- `inactive`：长时间无有效 session，进入历史。

### 4.2 Worker Session 状态

Worker Session 是调度权威状态：

| 状态 | 含义 | 是否可调度 |
| --- | --- | --- |
| `online` | tunnel 存在，租约有效，未 drain，generation 最新 | 是 |
| `draining` | 仍在线但不接新任务，等待当前任务结束 | 否 |
| `suspect` | 心跳接近超时或连接异常但未过 grace window | 否，除非明确允许降级 |
| `offline` | 租约超时或连接关闭 | 否 |
| `replaced` | 同 logical key 新 session 注册，旧 session 被 fencing | 否 |
| `stopped` | Worker 主动 unregister / graceful close | 否 |
| `expired` | 超过历史保留 TTL，只保留聚合统计或删除详情 | 否 |

### 4.3 失联原因码

| reason | 证据等级 | 说明 |
| --- | --- | --- |
| `graceful_shutdown` | 确定 | Worker 主动发送 unregister 或正常关闭 frame |
| `replaced_by_new_generation` | 确定 | 同 logical key 出现更高 generation，新 session 已 online |
| `drain_completed` | 确定 | 用户/系统 drain 后正常下线 |
| `transport_error` | 高 | gRPC stream 返回错误或 Server 侧观测到连接异常 |
| `heartbeat_timeout` | 中 | lease 超时，未收到明确 close 原因 |
| `server_restart_recovered` | 中 | Server 重启后根据持久会话恢复为未知离线 |
| `lease_expired_unknown` | 低 | 只有过期事实，无外部事件佐证 |
| `operator_marked_history` | 人工确认 | 运维手动归档或标记为历史 |

重要原则：**不要把 heartbeat timeout 直接叫做“异常宕机”**。它只能证明 Server 没有按时收到租约续约，原因可能是进程崩溃、网络分区、GC stop-the-world、节点重启、滚动发布或 Server 自身重启。

## 5. 注册与 fencing 流程

### 5.1 注册

1. Worker 发起 tunnel 并发送 `RegisterWorker`，包含 namespace/app/cluster/region/capabilities/labels/client_instance_id。
2. Server 计算 logical key。
3. Server 在事务中：
   - upsert Logical Worker；
   - 将同 logical key 下仍 active 的旧 session 标记为 `replaced`，写入 `replaced_by_worker_id`；
   - 创建新 Worker Session，生成 `worker_id`、`generation`、`fencing_token`；
   - 将新 session 放入内存连接路由表。
4. Server 返回 `WorkerRegistered { worker_id, generation, fencing_token, lease_seconds }`。

### 5.2 心跳

Worker 心跳必须携带：

```text
worker_id + generation + fencing_token + sequence
```

Server 校验：

- session 存在；
- fencing token 匹配；
- generation 是该 logical key 的最新 generation；
- sequence 单调递增或幂等可接受。

不满足则拒绝或忽略，并记录 `stale_worker_message` 事件。

### 5.3 任务日志与结果

任务 dispatch 时写入 `attempt_assignment_token`，绑定：

```text
instance_id + attempt_id + worker_id + generation + fencing_token
```

Worker 回传日志/结果时必须带 assignment token。Server 校验 token 后才接受。这样即使旧连接还活着，也不能覆盖新 session 或错误 attempt 的结果。

## 6. 失联与历史判定

### 6.1 Graceful 下线

SDK 在 `close()` / JVM shutdown hook / Rust Drop 等路径尽力发送 `UnregisterWorker` 或 `WorkerDisconnect`。Server 收到后：

- session -> `stopped`；
- reason -> `graceful_shutdown`；
- 当前未完成任务按策略进入 `retry_pending` / `lost_pending` / `wait_for_resume`。

### 6.2 异常断线

如果 gRPC stream onError/onCompleted 但没有 unregister：

- 立即将 session 标记为 `suspect` 或 `offline`；
- reason -> `transport_error`；
- 进入 short grace window，允许同 logical key 快速重连并标记旧 session 为 `replaced`。

### 6.3 心跳超时

后台 lease scanner 周期扫描 `lease_expires_at < now`：

- session -> `offline`；
- reason -> `heartbeat_timeout` / `lease_expired_unknown`；
- 调度器不再选择该 session；
- UI 放入“失联/待确认”区域，而不是直接归档历史。

### 6.4 被新会话替换

同 logical key 新 session 注册成功后，旧 active/suspect session：

- session -> `replaced`；
- reason -> `replaced_by_new_generation`；
- old connection route 从可投递路由表删除；
- 旧连接后续消息因 fencing 不匹配被拒绝。

### 6.5 历史归档

归档不是在线判定的一部分，而是保留策略：

- stopped/replaced session 可较快进入 history，例如 1-24 小时后默认折叠。
- heartbeat_timeout/transport_error 默认保留更久，例如 7 天，便于排障。
- 超过 retention 后可删除 session 明细，但保留聚合统计与 audit/event。

## 7. 存储模型建议

### 7.1 `worker_logical_instances`

```sql
id
namespace_id / namespace_name
app_id / app_name
cluster
region
client_instance_id
current_worker_id
current_generation
status
last_seen_at
created_at
updated_at
```

唯一索引：

```text
(namespace_name, app_name, cluster, region, client_instance_id)
```

### 7.2 `worker_sessions`

```sql
worker_id
logical_instance_id
connection_id
generation
fencing_token_hash
status
status_reason
status_evidence
lease_expires_at
last_heartbeat_at
last_sequence
capabilities_json
structured_capabilities_json
labels_json
master_json
connected_at
disconnected_at
replaced_by_worker_id
drain_requested_at
created_at
updated_at
```

索引：

- `(status, lease_expires_at)` 用于 lease scanner。
- `(logical_instance_id, generation)` 用于替换和历史查询。
- `(worker_id)` 作为 session 主键。

### 7.3 `worker_session_events`

追加式事件表，用于 UI timeline 和审计：

```sql
id
worker_id
logical_instance_id
event_type
reason
detail_json
created_at
```

当前已落地事件包括 `session_registered`、`session_replaced`、`stale_worker_message`、`lease_expired`、`graceful_shutdown`、`transport_error`；后续继续扩展 `heartbeat_renewed`、`dispatch_assigned`、`transport_closed`、`drain_requested`、`history_archived`。

> 2026-05-25 已落地 Slice B/C/D/E：`worker_logical_instances` / `worker_sessions` / `worker_session_events` 已进入迁移与 SQLite 兼容初始化；`WorkerRegistry` 在配置持久化仓储后会将注册、替换与心跳续租写入这些表；后台 lease scanner 会将过期 online session 标记为 `offline / lease_expired_unknown` 并写入 `lease_expired` 事件；Rust/Java SDK close 会发送 graceful unregister，Server 标记为 `stopped / graceful_shutdown`；dispatch 生成 assignment token，Rust/Java SDK 回传，Server 只接受当前 assignment token 的日志/结果。遵守项目既定约束：所有跨表关系都是软关联，不创建数据库外键。
>
> 2026-06-04 已补齐 Worker 可见性持久化：`worker_sessions` 追加保存 `capabilities_json`、`structured_capabilities_json`、`labels_json`、`master_json` 快照；注册、心跳、unregister、transport error 均更新/保留可观测快照；`/api/v1/workers` 合并 live registry 与持久层 online sessions，因此 server 重启后即使 Worker 尚未完成重连，也能先展示上次 online session 的结构化能力、labels 与 master/follower 状态。worker_pool scope 过滤必须读取结构化 labels/快照，不允许退回命名约定匹配。

## 8. SDK 配置建议

### 8.1 Java Spring Boot Starter

新增/调整默认解析：

```yaml
tikee:
  worker:
    client-instance-id: ${TIKEE_WORKER_CLIENT_INSTANCE_ID:}
    identity-mode: auto # auto | k8s_pod | container | stable_host_slot | ephemeral
    host-id: ${TIKEE_WORKER_HOST_ID:${HOSTNAME:}}
    instance-slot: ${TIKEE_WORKER_INSTANCE_SLOT:default}
    lifecycle-mode: session-per-process
```

推荐生产配置：

```yaml
env:
  - name: KUBERNETES_POD_UID
    valueFrom:
      fieldRef:
        fieldPath: metadata.uid
  - name: KUBERNETES_POD_NAME
    valueFrom:
      fieldRef:
        fieldPath: metadata.name
```

默认可以优先使用 Pod UID，避免 Deployment 中 Pod 名复用造成歧义；需要按 StatefulSet ordinal 聚合时由用户显式改成 Pod name/ordinal。

### 8.2 Rust / Go / Python / Node SDK

所有 SDK 保持同一语义：

- `worker_id` 只读，由 Server 下发。
- `client_instance_id` 是稳定 hint，可自动从环境变量推导。
- SDK 尽力 graceful unregister，但不能依赖它作为唯一判定。
- SDK 应支持重连退避和重新注册，重连后接受新的 `worker_id/generation/fencing_token`。

## 9. Web UI 与运维体验

Worker 集群页面应分层：

1. **Online Sessions**：当前可调度 Worker session。
2. **Degraded / Suspect**：租约异常、连接错误、未确认原因。
3. **Logical Workers**：按 `client_instance_id` 聚合显示当前 generation、最近 N 次 session、重启次数、最后下线原因。
4. **History**：默认折叠，可按 app/namespace/cluster/reason/时间过滤。

每个 Worker 详情页显示：

- logical key；
- current worker_id/generation；
- session timeline；
- last heartbeat；
- status reason/evidence；
- replacement chain；
- assigned/running attempts；
- stale message 拒绝记录。

UI 文案必须避免误导：

- `heartbeat_timeout` 展示为“租约超时，原因未确认”。
- `replaced_by_new_generation` 展示为“已被同一实例的新连接替换”。
- `graceful_shutdown` 展示为“客户端正常下线”。

## 10. 调度与任务恢复策略

调度器只选择：

```text
session.status == online
AND session.generation == logical.current_generation
AND lease_expires_at > now
AND drain_requested_at IS NULL
AND capabilities/labels match
```

已派发任务遇到 Worker 失联：

1. 短 grace window 内等待同 logical worker 重连。
2. 如果任务支持 resume/checkpoint 且 Worker WAL 能提供 attempt token，可恢复。
3. 否则按 job retry policy 重新入队或标记失败。
4. 所有结果必须通过 assignment token 校验，避免旧 session 迟到结果覆盖新 attempt。

## 11. 指标与告警

建议指标：

```text
tikee_worker_sessions{status,reason,namespace,app}
tikee_worker_logical_instances{status,namespace,app}
tikee_worker_session_replacements_total{namespace,app}
tikee_worker_lease_expirations_total{reason,namespace,app}
tikee_worker_stale_messages_total{kind,namespace,app}
tikee_worker_generation{namespace,app,client_instance_id}
```

建议告警：

- 同 app 短时间大量 `heartbeat_timeout`：可能网络或 Server 压力。
- 同 logical worker 高频 replacement：可能 crash loop。
- stale message 激增：可能旧连接未断、代理乱序或 SDK bug。
- online session 低于 Worker Pool 最小副本数。

## 12. API 建议

新增或增强：

```text
GET /api/v1/workers/sessions
GET /api/v1/workers/logical-instances
GET /api/v1/workers/sessions/{worker_id}
GET /api/v1/workers/logical-instances/{id}/sessions
POST /api/v1/workers/sessions/{worker_id}:drain
POST /api/v1/workers/sessions/{worker_id}:mark-history
```

返回字段包含：status、reason、evidence、generation、client_instance_id、logical_instance_id、lease_expires_at、last_heartbeat_at、replaced_by_worker_id。

## 13. 分阶段实施计划

### Slice A：协议与存储基础

- 扩展 WorkerRegistered：返回 generation、fencing_token、lease_seconds。
- 新增 worker logical/session/event 存储表，保持无外键软关联。
- 注册时 upsert logical instance，创建 session，替换旧 generation。

### Slice B：租约与状态机

- Heartbeat 携带 generation/fencing token。
- Lease scanner 将过期 session 标记为 offline/suspect。
- gRPC stream close/error 写 session event。
- SDK close 尽力发送 unregister。

### Slice C：调度 fencing

- Dispatch 只选择最新 online session。
- Dispatch assignment token 绑定 worker session。
- Log/result 校验 assignment token，拒绝 stale session。

### Slice D：Java/Rust SDK 环境感知 identity

- Java Starter 默认解析显式 env、K8s Pod UID、容器身份、裸机/VM host id + instance slot。
- Rust SDK 对齐同一 `client_instance_id` 解析策略。
- 文档给出 Deployment/StatefulSet 推荐配置。

### Slice E：Web 与观测

- Worker 页面拆分 Online / Suspect / Logical / History。
- 增加 session timeline 和 replacement chain。
- 增加指标与告警规则。

### Slice F：重启后可见性快照

- 注册/心跳时持久化 capabilities、structuredCapabilities、labels、master 快照。
- Worker 列表 API 合并 live registry 与持久 online sessions，live 优先、DB 快照兜底。
- scope/worker_pool 过滤使用 namespace/app/cluster/region/labels/structuredCapabilities 等结构化字段，不允许依赖 clientInstanceId 或名称约定。
- UI 按 namespace/app、cluster/region 分组展示 node 列表，调度队列放到独立二级页或抽屉。

## 14. 验证计划

- 单元测试：注册同 logical key 两次，旧 session 变 `replaced`，新 generation 可调度。
- 单元测试：旧 fencing token 心跳/结果被拒绝。
- 集成测试：Worker 正常 close -> `graceful_shutdown`。
- 集成测试：停止心跳 -> `lease_expired_unknown`，不会误标为 graceful。
- 集成测试：新 Worker 重连 -> 旧 session `replaced_by_new_generation`。
- Java Starter 测试：默认 client_instance_id 从 env/K8s/host-slot 推导；显式配置优先。
- Web 测试：默认只显示 online，history 折叠且可过滤。
- 集成测试：启动 Worker 后重启 server，在 Worker 重连前 `/api/v1/workers` 仍能读取 DB 快照展示 online session 能力与 master 状态；Worker 重连后 live registry 覆盖 DB 快照。
- 回归测试：worker_pool 过滤对 live Worker 与持久化快照 Worker 结果一致。

## 15. 关键决策

1. `worker_id` 继续代表 ephemeral session，不改成稳定机器 ID。
2. `client_instance_id` 只作为 logical grouping hint，不作为调度或写入权威。
3. 对失联原因做证据分级，不能把心跳超时武断判为异常宕机。
4. 使用 generation + fencing_token 保护旧连接、迟到消息和僵尸 session。
5. UI 默认面向运维：当前在线优先，历史折叠，异常待确认单独突出。
