# Raft 与 Worker Master 选举现状审计报告

**日期:** 2026-05-30  
**范围:** tikee server 集群 Raft、worker 服务集群 master 选举、调度有序性与无额外锁机制约束。  
**目标:** 将现有半闭环 Raft/worker 生命周期能力升级为生产级自主 master 选举能力。

## 1. 现状结论

### 1.1 tikee server 集群

当前 server 侧已经具备 raft-rs 基础设施：`RawNode`、ticker runtime、HTTP transport、HardState/log/snapshot/ConfState 持久化、leader fencing token，以及调度 ownership gate。调度、worker dispatch、告警 retry 均依赖 `ClusterStatus.can_schedule`，非 owner 节点不会执行 ownership-sensitive loop。

但当前生产 runtime 启动后不会自主 campaign。代码注释和启动 detail 明确说明“no campaign”。现有选主测试依赖测试 harness 手动调用 `RawNode::campaign()`，只能证明 raft-rs 可以选出 leader，不能证明生产 runtime 会自主产生 master。

**结论:** server Raft 是基础可用但未生产闭环；缺自主选主触发与生产式多节点验证。

### 1.2 worker 服务集群

当前 worker 侧具备 session lifecycle：注册、logical instance、generation、fencing token、heartbeat lease、replaced/stopped/offline 状态。这能解决单个逻辑实例重启、旧连接心跳被 fencing 的问题。

但 worker 注册协议、server registry、worker lifecycle repository 中都没有 worker cluster election、leader term、worker master role 或投票状态。worker `cluster` 字段只是元数据/筛选条件，不代表 master 选举域。

**结论:** worker 服务集群 master 选举尚未实现。

## 2. 生产级缺口清单

### P0-1 Server Raft 自主选主

- 启动后必须由 raft runtime 自主进入 election，不依赖人工 API 或测试 harness。
- leader 只有在 raft-rs role=Leader、term>0、leader fencing token 成功持久化后，才可 `can_schedule=true`。
- follower/candidate/pre-candidate 均不得调度。
- 必须有多节点测试证明不手动 campaign 也能选出唯一 leader。

### P0-2 Worker 集群结构化 master 选举

- worker 注册必须结构化声明 worker cluster election 信息，而不是依赖字符串约定。
- 同一个 namespace/app/cluster/region 构成 worker election domain。
- 每个 domain 内在线且 lease 有效的 worker 必须能确定唯一 master。
- master 必须带 term/fencing/version 信息，worker 列表和 dispatch 侧都能看到。
- worker 下线、停止、lease 过期或新 generation 替换时，必须可重新选主。

### P0-3 调度有序性

- server 侧调度只能由 server Raft leader 执行。
- worker 侧普通单实例任务派发必须优先落到对应 worker domain 的 master，避免同一 worker 服务集群内多节点并发接单造成顺序混乱。
- broadcast fan-out 可以仍向所有匹配 worker 发，但每个 worker session 的 assignment token/fencing 仍必须有效。

### P0-4 SDK/Demo 闭环

- Java SDK 注册信息需要携带 worker cluster election 声明。
- Spring demo 默认启用结构化 worker master election，并在日志中打印 election domain 与当前注册信息。
- Rust SDK 暂保持可编译状态，不做深度行为改造。

### P0-5 文档与验证

- 更新总设计文档/路线图状态。
- 增加覆盖报告状态说明。
- 增加针对 server 自主选主、worker master 变更、dispatch master 优先的测试。

## 3. 验收标准

1. `rtk cargo test -p tikee-server raft -- --nocapture` 通过，且包含无手动 campaign 的自主选主测试。
2. `rtk cargo test -p tikee-server worker -- --nocapture` 通过，且包含 worker master 初选、故障切换、replacement fencing 测试。
3. Java SDK/demo 测试通过，注册请求包含结构化 election 信息。
4. worker API 返回 master 状态、term/fencing/domain 信息。
5. 调度路径优先使用 worker cluster master。
6. 文档和覆盖报告明确标记该能力已闭环或注明验证边界。
