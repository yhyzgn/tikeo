# tikeo — 下一代分布式任务调度平台架构设计

> **Rust 原生 | 单二进制 | gRPC 标准 | Cloud Native First | 零历史债**
> 本设计保留合理的架构、协议、组件、部署、技术栈和路线图设计，项目名称统一为 **tikeo**，并增强 PowerJob / xxl-job 源码级剖析、K8s 公共服务化问题、全新开发必要性与创新功能点。

---

## 1. 项目概述

### 1.1 什么是 tikeo

tikeo 是一个用 Rust 从零构建的分布式任务调度与计算平台，目标是**完全覆盖 PowerJob 的全部功能特性**，同时在性能、易用性、部署体验和安全模型上实现质的飞跃。

### 1.2 为什么要从 0 开发

经过 xxl-job 与 PowerJob 源码调研后，结论明确：**xxl-job 是能力上限不足，PowerJob 是能力堆叠后架构债过重**。二者都不适合作为企业平台统一调度底座。

| 痛点 | xxl-job 现状 | PowerJob 现状 | tikeo 目标 |
|------|--------------|---------------|----------------|
| 调度能力 | 核心持久调度只有 CRON / FIX_RATE；FIX_DELAY 在源码枚举中仍是注释状态；任务依赖只是 child_jobid 串联 | 调度方式更多，但 CRON/工作流/秒级任务由多套路径实现，调度责任散落在 Server 与 Worker | 统一 Schedule / Trigger Event 模型，覆盖 CRON、FIX_RATE、FIX_DELAY、API、延迟、一次性、日历调度 |
| 执行模型 | 单机、分片广播为主，无 MapReduce 内核，无真正 DAG 工作流 | 有 STANDALONE / BROADCAST / MAP / MAP_REDUCE / DAG，但状态、通信和本地持久化耦合重 | 覆盖 PowerJob 执行模型，并把 MapReduce、DAG、长运行任务做成可恢复、可观测状态机 |
| 公共服务化 | Admin 反向访问 Executor，Executor 必须暴露入站端口 | Server 反向访问 Worker，Worker 也必须绑定端口并上报 external address | Worker 主动建立 gRPC/HTTP2 长连接隧道，Server 不回连业务 Pod，天然适配 K8s/Docker/NAT/多级网关/跨集群 |
| 集群协调 | MySQL `FOR UPDATE` 全局调度锁 | DB 锁 + `currentServer` + PING 选主，不是共识 | Raft / lease shard / fencing token，调度归属可验证、可恢复 |
| 部署体验 | Spring Boot Admin + MySQL + Executor 端口；能力简单但仍非单二进制 | Java 8+、Spring Boot、Undertow、Akka、Vert.x、MySQL、本地 H2、多端口 7700/10086/10010/10077 | Server/Worker 均容器优先，K8s/Docker/Compose/Nomad/systemd 全支持；单二进制、单端口、开发态 SQLite、生产态 MySQL/PostgreSQL/CockroachDB |
| 安全边界 | 默认 token、GLUE/Shell/Python/Node/PowerShell 在宿主执行 | Groovy 决策、SQL Processor、HTTP Processor、脚本下载执行、customQuery 等攻击面大 | 默认 mTLS/RBAC/OIDC/审计；WASM/子进程沙箱；URL policy；参数化 SQL |
| 可观测性 | Executor 本地日志，Admin 轮询读取 | Worker 内存队列批量上报，队列满或 Server 不可用会丢日志 | gRPC 流式日志、背压、OTLP、Prometheus、审计事件、事故回放 |
| 可维护性 | 架构简单但功能空间太窄 | Akka/HTTP/MU 三协议、JPA/H2、本地文件交换、历史兼容层多 | 协议、状态机、存储、执行沙箱从一开始按企业平台设计 |

因此 tikeo 不是“PowerJob 的 Rust 版本”，也不是“xxl-job 加功能”。它是面向企业平台公共调度服务的一次重新建模：**用更少的核心抽象承载更多、更可靠、更安全的能力**。
### 1.3 源码调研后的核心结论

本设计不再只以 PowerJob 为单一参照，而是把 xxl-job 与 PowerJob 都作为反例基线：

| 结论 | xxl-job 源码表现 | PowerJob 源码表现 | tikeo 设计取舍 |
|------|------------------|-------------------|--------------------|
| 公共服务化的最大阻碍是 Worker/Executor 入站可达 | Admin 通过 HTTP 调 Executor 内嵌 Netty 服务 | Server 通过 AKKA/HTTP/MU 直接调 Worker 上报地址 | Worker 主动建立 gRPC 双向流，Server 不回连业务 Pod |
| DB 锁不是调度集群共识 | `xxl_job_lock` + `FOR UPDATE` 全局锁 | `oms_lock` + `currentServer` + PING 选主 | TiKV raft-rs / lease shard / fencing token |
| 内存时间轮只能做加速，不能做事实源 | 60 秒 ringData，Admin 重启即丢 | InstanceTimeWheelService 承担延迟派发 | trigger_event 持久化，内存轮只做 near-time cache |
| 动态脚本/SQL/HTTP 参数必须有安全边界 | GLUE/Shell/Python/Node/PowerShell 宿主执行 | Groovy/SQL/HTTP/脚本下载执行攻击面大 | 多语言脚本运行时 + WASM/子进程/容器沙箱、URL policy、参数化 SQL、审计 |
| 工作流必须是一等状态机 | child_jobid 不是 DAG | DAG 存在但状态散落、Groovy 决策风险高 | workflow_event + typed context + safe expression |

因此，tikeo 的目标不是“复刻 PowerJob”，而是保留其有价值的功能模型（多调度方式、MapReduce、DAG、官方处理器），同时替换掉不适合云原生公共服务的通信、选举、状态、安全和可观测性设计。
关键源码依据（用于支撑后文设计取舍）：

- xxl-job 调度类型与 DB 锁：`xxl-job-admin/src/main/java/com/xxl/job/admin/tikeo/type/ScheduleTypeEnum.java`、`xxl-job-admin/src/main/resources/mapper/XxlJobLockMapper.xml`、`JobScheduleHelper.java`
- xxl-job Executor 反向调用与 child_jobid：`xxl-job-core/src/main/java/com/xxl/job/core/executor/XxlJobExecutor.java`、`xxl-job-admin/src/main/java/com/xxl/job/admin/tikeo/complete/JobCompleter.java`
- PowerJob 多协议与端口：`powerjob-server-starter/src/main/resources/application.properties`、`PowerTransportService.java`、`powerjob-remote-impl-akka/.../package-info.java`
- PowerJob Worker 入站地址与 DB 选举：`PowerJobWorker.java`、`ServerElectionService.java`、`DatabaseLockService.java`
- PowerJob 调度、H2、日志与 Groovy：`CoreScheduleTaskManager.java`、`PowerScheduleService.java`、`ConnectionFactory.java`、`OmsLogHandler.java`、`DecisionNodeHandler.java`

### 1.4 核心设计原则

1. **Simplicity over Flexibility** — 能用一种方式解决的，不用两种
2. **Protocol as Contract** — gRPC protobuf 即接口文档，即多语言 SDK
3. **Single Binary** — 编译产物为一个可执行文件，`./tikeo serve` 即启动
4. **Memory Safe by Default** — Rust 所有权模型消除整类内存 bug
5. **Cloud Native First** — K8s/Docker/容器部署是一等能力，Server 与 Worker 可部署在不同容器、namespace、集群、VPC 或云厂商中
6. **Zero Trust** — 默认 TLS + mTLS、RBAC、审计日志

---

## 2. 功能覆盖与竞品对照

> 设计目标：不是简单“100% 覆盖 PowerJob”，而是以 xxl-job 和 PowerJob 的源码事实为基线，保留 PowerJob 中有价值的功能模型，同时修正二者在通信、调度、工作流、安全、可观测性上的架构缺陷。

### 2.1 调度能力

| 功能 | xxl-job | PowerJob | tikeo | 增强重点 |
|------|---------|----------|--------------|----------|
| CRON 表达式 | ✅ | ✅ | ✅ | 秒级 CRON、时区、DST 策略、日历排除 |
| 固定频率 FIX_RATE | ✅ 秒级 | ✅ 毫秒参数，但频繁任务由 Worker 本地发射 | ✅ | Server 统一生成 Trigger Event，支持 jitter、防惊群、catch-up/skip/latest-only |
| 固定延迟 FIX_DELAY | ⚠️ 文档提及，核心枚举注释 | ✅ | ✅ | 基于上次完成事件生成下一次，支持指数退避 |
| API/手动触发 | ✅ | ✅ | ✅ | REST/gRPC/CLI/Webhook/EventBridge 统一触发入口 |
| 延迟任务 | ❌ | ⚠️ 主要依赖时间轮/实例延迟，不适合大规模长期延迟 | ✅ | 持久化 delay queue，内存时间轮只做 near-time cache |
| 一次性未来任务 | ⚠️ 非一等模型 | ✅ | ✅ | 一等调度类型，精确到毫秒，支持取消与重排 |
| Daily Time Interval | ❌ | ✅ | ✅ | 作为 Calendar Schedule 插件化能力 |
| Misfire 策略 | ✅ 基础策略 | ⚠️ 分散在不同调度路径 | ✅ | DO_NOTHING、FIRE_ONCE、CATCH_UP_LIMITED、RESCHEDULE |
| 生命周期窗口 | ⚠️ 弱 | ✅ | ✅ | start/end、维护窗口、冻结窗口、节假日策略 |

### 2.2 执行模式

| 功能 | xxl-job | PowerJob | tikeo | 增强重点 |
|------|---------|----------|--------------|----------|
| 单机执行 | ✅ | ✅ | ✅ | 幂等 token、attempt 追踪、worker lease |
| 广播执行 | ✅ 分片广播 | ✅ | ✅ | 按 tag/region/version/tenant 条件广播 |
| 分片任务 | ✅ | ✅ | ✅ | 动态分片、失败分片重平衡 |
| Map | ❌ | ✅ | ✅ | 可恢复 map task，分片结果可追踪 |
| MapReduce | ❌ | ✅ | ✅ | 流式 reduce、spill-to-disk、结果分片、checkpoint |
| 工作流 DAG | ❌ child_jobid 串联触发 | ✅ DAG/DECISION/NESTED_WORKFLOW | ✅ | 事件溯源状态机、安全表达式、强类型上下文、人工节点、补偿节点 |
| 长运行任务 | ⚠️ | ⚠️ | ✅ | 心跳租约、续租、checkpoint/resume、优雅取消 |
| 任务排队 | ❌ 能力弱 | ⚠️ 超限容易失败或转本地复杂状态 | ✅ | 每租户/每 worker pool 队列、优先级、限流、背压 |

### 2.3 处理器类型

| 功能 | xxl-job | PowerJob | tikeo | 安全/体验增强 |
|------|---------|----------|--------------|----------------|
| Java Bean / SDK | ✅ | ✅ | ✅ | 提供 Java 兼容 SDK，同时多语言一致协议 |
| Rust 原生处理器 | ❌ | ❌ | ✅ | primary SDK，零成本抽象 |
| Go/Python/Node SDK | ❌ | ❌ | ✅ | protobuf/gRPC 自动生成 + 手写 ergonomic SDK |
| HTTP 调用 | ⚠️ 常见用法 | ✅ 官方处理器 | ✅ | URL allowlist/denylist、内网 IP 阻断、重试、熔断、签名 |
| Shell/Python/Node/PHP/PowerShell | ✅ GLUE 脚本 | ✅ Shell/Python/CMD/PowerShell | ✅ | 统一 Script Processor，多语言运行时，子进程/容器沙箱、CPU/内存/文件/网络限制 |
| SQL 执行 | ❌ | ✅ | ✅ | 参数化模板、数据源白名单、dry-run、审批、审计 |
| 文件清理 | ❌ | ✅ | ✅ | 路径白名单、dry-run、最小权限 |
| Groovy/动态脚本 | ✅ GLUE_GROOVY | ✅ Groovy 决策 | ✅ 受控支持 | 支持 Python/Node/Shell/PowerShell/JavaScript/Rhai/WASM 等；默认禁用宿主级反射/系统调用，强制沙箱、签名、审批与审计 |
| 外部 JAR/容器 | ⚠️ | ✅ JAR Container | ✅ | 优先 WASM/容器沙箱，版本化和签名校验 |
| gRPC 调用 | ❌ | ❌ | ✅ | 任意 gRPC 服务可作为任务处理器 |
| Webhook | ⚠️ | ⚠️ | ✅ | 入站/出站 Webhook，HMAC 签名和重放保护 |

### 2.4 管理与平台能力

| 功能 | xxl-job | PowerJob | tikeo | 增强重点 |
|------|---------|----------|--------------|----------|
| Web 控制台 | ✅ 传统模板页 | ✅ Vue 控制台 | ✅ 嵌入式前端 | 单二进制内置、暗色模式、移动端适配 |
| OpenAPI | ✅ | ✅ | ✅ | REST OpenAPI 3.1 + gRPC reflection |
| 实时日志 | ⚠️ 轮询 Executor 本地文件 | ⚠️ Worker 队列批量上报，可能丢 | ✅ | gRPC 流式推送、背压、对象存储归档 |
| 工作流可视化 | ❌ | ✅ | ✅ | 拖拽 + YAML/JSON 双模式，diff、仿真、回放 |
| 用户权限 | ✅ 基础用户/权限 | ✅ V5.x 权限 | ✅ | RBAC + OIDC + Service Account + API Token |
| 多租户 | ⚠️ 执行器/分组弱隔离 | ✅ Namespace | ✅ | namespace/app/worker pool/secret/quota 全链路隔离 |
| 告警通知 | ✅ 邮件等 | ✅ 邮件/钉钉/Webhook | ✅ | 邮件/飞书/钉钉/企微/Slack/PagerDuty/Webhook，去重和静默 |
| 指标监控 | ⚠️ 弱 | ⚠️ Actuator/自接 | ✅ | Prometheus + OTLP 原生导出 |
| 审计日志 | ⚠️ 弱 | ⚠️ 不完整 | ✅ | 所有 CRUD、触发、取消、审批、密钥使用可审计 |
| GitOps/IaC | ❌ | ❌ | ✅ | YAML、CRD、Terraform Provider、变更 diff |
## 3. 架构设计

### 3.1 总体架构

```mermaid
graph TB
    subgraph Cluster["tikeo Cluster"]
        direction TB

        subgraph S1["Server #1 (Leader)"]
            SCH1["Tikeo"]
            WF1["Workflow Engine"]
            GW1["gRPC+HTTP Gateway"]
            UI1["Web UI (embedded)"]
        end

        subgraph S2["Server #2 (Follower)"]
            SCH2["Tikeo"]
            WF2["Workflow Engine"]
            GW2["gRPC+HTTP Gateway"]
            UI2["Web UI (embedded)"]
        end

        subgraph S3["Server #N (Follower)"]
            SCH3["Tikeo"]
            WF3["Workflow Engine"]
            GW3["gRPC+HTTP Gateway"]
            UI3["Web UI (embedded)"]
        end

        S1 ---|Raft Consensus| S2
        S2 ---|Raft Consensus| S3
        S3 ---|Raft Consensus| S1
    end

    DB[(Database<br/>SQLite / MySQL /<br/>PostgreSQL / CockroachDB)]
    Cluster --- DB

    Gateway>Client / Browser<br/>REST + gRPC-Web]
    Gateway --- GW1

    Cluster ---|gRPC h2| WA["Worker A<br/>Rust SDK"]
    Cluster ---|gRPC h2| WB["Worker B<br/>Go SDK"]
    Cluster ---|gRPC h2| WC["Worker C<br/>Python SDK"]

    subgraph WA_Internals[" "]
        WA_E["Executor Pool"]
        WA_W["WASM Sandbox"]
        WA_S["Script Sandbox<br/>Python / Node / Shell"]
    end
    WA --- WA_Internals

    subgraph WB_Internals[" "]
        WB_E["Executor Pool"]
        WB_S["Script Sandbox<br/>Python / Node / Shell"]
        WB_H["HTTP Processor"]
    end
    WB --- WB_Internals

    subgraph WC_Internals[" "]
        WC_E["Executor Pool"]
        WC_H["HTTP Processor"]
    end
    WC --- WC_Internals
```

### 3.2 核心设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 通信协议 | **gRPC (HTTP/2)** | 双向流、多语言 SDK 自动生成、TLS 原生；Worker 主动出站连接可穿透 NAT、Ingress、Gateway、Service Mesh 和跨集群网络层级 |
| 共识机制 | **Raft** (内置) | 取代 PowerJob 的数据库锁选举。更可靠、更快速、不依赖数据库 |
| 存储层 | **SeaORM** + 多数据库 | 支持 SQLite / MySQL / PostgreSQL / CockroachDB，异步原生，生产就绪 |
| 沙箱隔离 | **WASM (Wasmtime) + 子进程/容器沙箱** | WASM 承载高安全插件；多语言动态脚本通过受限子进程或容器运行，统一资源、文件、网络和审计策略 |
| 前端 | **编译时嵌入** (include_dir!) | 单二进制内置前端静态资源，无需独立部署 |
| 序列化 | **Protocol Buffers** | 高性能、强类型、多语言支持 |
| 日志 | **tracing + OTLP** | 结构化日志、分布式追踪、多后端导出 |

---

## 4. 组件设计

### 4.1 tikeo Server

Server 是平台的核心，承担调度、工作流编排、集群管理、API 网关四大职责。

#### 4.1.1 模块架构

```mermaid
graph LR
    subgraph Server["tikeo-server"]
        MAIN["main.rs<br/>CLI + 启动"]
        CFG["config.rs<br/>TOML 配置"]
        SRV["server.rs<br/>服务组装"]

        subgraph Tikeo["tikeo/"]
            CRON["cron.rs<br/>CRON 解析"]
            TW["time_wheel.rs<br/>时间轮"]
            DQ["delay_queue.rs<br/>持久化延迟"]
            DSP["dispatcher.rs<br/>分发策略"]
        end

        subgraph Workflow["workflow/"]
            DAG["dag.rs<br/>DAG 引擎"]
            CTX["context.rs<br/>上下文"]
            COND["condition.rs<br/>条件分支"]
            SUB["sub_workflow.rs<br/>子工作流"]
        end

        subgraph Cluster["cluster/"]
            RAFT["raft.rs<br/>Raft 状态机"]
            MEM["membership.rs<br/>成员管理"]
        end

        subgraph Storage["storage/"]
            ORM["SeaORM 抽象层"]
            SQLITED["SQLite Driver"]
            MYSQLD["MySQL Driver"]
            PGD["PostgreSQL Driver"]
            CRDBD["CockroachDB Driver"]
        end

        subgraph Gateway["gateway/"]
            GRPC["grpc.rs<br/>gRPC 服务"]
            HTTP["http.rs<br/>HTTP REST API"]
            OPENAPI["openapi.rs<br/>OpenAPI 3.1"]
            WS["websocket.rs<br/>SSE/WebSocket 实时推送"]
        end

        subgraph Auth["auth/"]
            RBAC["rbac.rs"]
            OIDC["oidc.rs"]
        end

        subgraph Alert["alert/"]
            CH["channel.rs"]
            RULE["rule.rs"]
        end

        subgraph Obs["observability/"]
            MET["metrics.rs"]
            TRC["trace.rs"]
        end

        WEB["web/<br/>React + Ant Design 管理控制台"]
    end
```

#### 4.1.2 调度器内部流程

```mermaid
flowchart TD
    A[DB: 加载 Job 定义] --> B{Schedule Type?}

    B -->|CRON| C1[CRON Engine<br/>计算下次触发时间]
    B -->|FIXED_RATE| C2[Fixed Rate Engine<br/>固定间隔]
    B -->|FIXED_DELAY| C3[Fixed Delay Engine<br/>上次完成后延迟]
    B -->|API| C4[等待 API 触发]

    C1 --> D{延迟 ≤ 1h?}
    C2 --> D
    C3 --> D

    D -->|是| E1[写入内存时间轮<br/>纳秒精度]
    D -->|否| E2[写入持久化延迟队列<br/>DB 存储, 秒级精度]

    E1 --> F[Trigger Queue<br/>tokio::mpsc bounded]
    E2 -->|定时扫描| F
    C4 --> F

    F --> G[Dispatcher<br/>选择目标 Worker]

    G --> H{分发策略}
    H -->|Random| I1[随机选择 Worker]
    H -->|RoundRobin| I2[轮询选择 Worker]
    H -->|Tag/Region| I3[按标签筛选 Worker]
    H -->|Specify| I4[指定 Worker 地址]

    I1 --> J[gRPC DispatchTask]
    I2 --> J
    I3 --> J
    I4 --> J

    J --> K[Worker 执行任务]

    K --> L{执行结果}
    L -->|成功| M1[更新 instance = SUCCESS]
    L -->|失败且可重试| M2[重试 → 重新分发]
    L -->|失败且不可重试| M3[更新 instance = FAILED<br/>触发告警]
```

#### 4.1.3 工作流引擎 DAG 执行

```mermaid
flowchart TD
    START([Workflow Trigger]) --> PARSE[解析 DAG 定义]

    PARSE --> TOPO[拓扑排序<br/>确定执行层级]

    TOPO --> LAYER{遍历每一层节点}

    LAYER --> EVAL{评估条件表达式}
    EVAL -->|条件为真| RUN_A[执行节点 A]
    EVAL -->|条件为假| SKIP[跳过该分支]

    RUN_A --> ASYNC1[异步并行执行<br/>同层无依赖节点]
    ASYNC1 --> WAIT[等待同层全部完成]

    WAIT --> MERGE_CTX[合并上下文<br/>KV Store]

    MERGE_CTX --> LAYER

    LAYER -->|所有层完成| DONE([Workflow Complete])

    ASYNC1 -->|超时| TIMEOUT[节点超时处理]
    TIMEOUT --> FAIL_STRAT{失败策略}
    FAIL_STRAT -->|Retry| RUN_A
    FAIL_STRAT -->|Skip| WAIT
    FAIL_STRAT -->|Pause| PAUSE[暂停工作流<br/>等待人工介入]
    FAIL_STRAT -->|Callback| CB[触发回调<br/>后继续/终止]

    subgraph 增强 vs PowerJob
        E1[✅ 条件分支]
        E2[✅ 循环节点 for/while]
        E3[✅ 子工作流嵌套]
        E4[✅ 全局 + 单节点超时]
        E5[✅ 任务排队不直接失败]
    end
```

**超越 PowerJob 的工作流特性**：

| 特性 | PowerJob | tikeo |
|------|----------|----------|
| DAG 依赖 | ✅ | ✅ |
| 条件分支 | ❌ | ✅ (基于上下文的条件表达式) |
| 循环节点 | ❌ | ✅ (for/while 循环控制) |
| 子工作流 | ❌ | ✅ (工作流嵌套调用) |
| 节点超时 | 部分 | ✅ (全局 + 单节点超时) |
| 失败策略 | 重试 | ✅ 重试 / 跳过 / 暂停 / 回调 |
| 上下文传递 | KV HashMap | ✅ 强类型上下文 + JSON Schema 校验 |
| 任务排队 | ❌ (超限直接失败) | ✅ (可配置队列容量 + 优先级) |

### 4.2 tikeo Worker (SDK)

Worker 是嵌入业务应用的客户端库，负责接收任务、执行处理器、上报状态。

#### 4.2.1 Worker 内部架构

```mermaid
flowchart TD
    subgraph Worker["Worker SDK"]
        GRPC_CLI["gRPC Client<br/>双向流连接"]

        subgraph Pool["Executor Pool"]
            T1["Task Slot 1"]
            T2["Task Slot 2"]
            TN["Task Slot N"]
        end

        subgraph Proc["Processor Chain"]
            BP["Built-in Processor<br/>HTTP / SQL / Script"]
            WASM["WASM Sandbox<br/>用户自定义代码"]
            SCRIPT["Script Sandbox<br/>Python / Node / Shell / PowerShell / Rhai"]
            NATIVE["Native Processor<br/>Rust trait impl"]
        end

        STATUS["Status Reporter<br/>异步批量上报"]
        LOG["Log Streamer<br/>gRPC Client Stream<br/>背压控制"]
        HB["Heartbeat<br/>10s 间隔"]
        METRIC["Metrics Collector<br/>CPU / 内存 / 磁盘"]
    end

    GRPC_CLI -->|接收任务| Pool
    Pool -->|路由| Proc
    Proc -->|执行结果| STATUS
    Proc -->|执行日志| LOG

    HB -->|定时| GRPC_CLI
    METRIC -->|附加| HB
    STATUS -->|上报| GRPC_CLI
    LOG -->|流式| GRPC_CLI
```

#### 4.2.2 多语言 SDK 策略

通过 gRPC protobuf 定义 Worker 协议，自动生成各语言 SDK：

```
proto/tikeo/worker/v1/
├── worker.proto          # Worker 注册、心跳、任务接收
├── task.proto            # 任务定义、状态、结果
├── processor.proto       # 处理器协议
└── workflow.proto        # 工作流上下文
```

SDK 与示例统一目录（强约束）：

```text
sdks/
├── rust/
│   └── tikeo/           # Rust Worker SDK crate (tonic)
├── java/
│   ├── tikeo/                              # 原生 Java SDK
│   ├── tikeo-spring/                       # Spring 7 / Boot 4 集成
│   ├── tikeo-spring5/                      # Spring 5 / Boot 2 兼容适配
│   ├── tikeo-spring6/                      # Spring 6 / Boot 3 兼容适配
│   ├── tikeo-spring-boot2-starter/         # Spring Boot 2 starter
│   ├── tikeo-spring-boot3-starter/         # Spring Boot 3 starter
│   └── tikeo-spring-boot-starter/          # Spring Boot 4 starter
├── go/
│   └── tikeo/                      # Go Worker SDK module (official gRPC/protobuf)
├── python/
│   └── tikeo-python-sdk/           # 规划
└── nodejs/
    └── tikeo-nodejs-sdk/           # 规划

examples/
├── rust/
│   └── worker-demo/                    # Rust SDK demo worker / task processor
├── java/
│   ├── spring-boot2-worker-demo/       # Java Spring Boot 2 demo app，Gradle 构建，Java 17+
│   ├── spring-boot3-worker-demo/       # Java Spring Boot 3 demo app，Gradle 构建，Java 17+
│   └── spring-boot4-worker-demo/       # Java Spring Boot 4 demo app，Gradle 构建，Java 17+
├── go/
│   └── worker-demo/                    # Go SDK demo worker
├── python/
│   └── worker-demo/                    # Python SDK demo worker
└── nodejs/
    └── worker-demo/                    # Node.js SDK demo worker
```

规则：
- `sdks/` 只存放 SDK 实现，不放业务 demo；每个 SDK 必须位于 `sdks/<language>/<sdk-name>/`，不能直接放在语言目录根下。
- `examples/` 按 `sdks/` 的语言结构一一对应存放可运行 demo 项目；每个 demo 必须位于 `examples/<language>/<demo-name>/`，并能单独构建/运行。
- 后续开发过程中，AI 开发者需要自行判断验证需要；当 SDK、Worker Tunnel、任务执行、工作流或跨语言集成链路需要端到端调试时，应主动创建/更新对应 `examples/<language>/...` demo，而不是等待用户显式要求。
- 运行配置仍放 `config/`，不得把 `examples/` 再作为配置目录使用。
- Rust SDK 已按规范迁移到 `sdks/rust/tikeo`，Cargo workspace 已同步调整。
- Go SDK 已落地到 `sdks/go/tikeo`，使用官方 `google.golang.org/grpc` 与 `google.golang.org/protobuf` 生成 Worker Tunnel 绑定；Go demo 位于 `examples/go/worker-demo`，默认 live 连接，显式设置 dry-run 才离线。
- 根 `Dockerfile` 只构建 tikeo 服务端镜像，不复制、不缓存、不构建 `sdks/` 与 `examples/`；SDK 与 Demo 必须作为独立构建产物验证。
- 独立发布约束：每个 SDK 必须可按语言生态独立发布；Rust SDK 不能依赖服务端 `crates/*` path dependency，必须内聚协议定义或依赖已发布协议包。
- Worker 注册约束：`worker_id` 必须由服务端生成并在 `WorkerRegistered` 下发；客户端只能上报可选 `client_instance_id` 作为实例提示，不能自行声明权威 ID。`worker_id` 语义上代表一次具体 Worker Tunnel session/incarnation，而不是长期稳定机器 ID。
- Worker 身份生命周期约束：生产环境需要区分 Worker Pool、Logical Worker Instance（`namespace/app/cluster/region/client_instance_id`）和 Worker Session（`worker_id/generation/fencing_token`）。失联判定必须按证据分级，心跳超时只能标记为租约过期/原因未确认，不能直接断言异常宕机。完整方案见 `design/worker-identity-lifecycle-design.md`。
- Worker 分发约束：`DispatchTask.processor_name` 是 SDK 侧处理器路由的显式字段；Job 定义与 Workflow job/map 节点均支持显式 `processor_name` 绑定，dispatcher 优先使用节点绑定，其次使用 Job 绑定，最后仅为历史数据回退到 `job_id`。
- Node 目录统一命名为 `nodejs`，避免和通用 node/graph 概念混淆。
- 代码组织约束：所有 Rust server/crates、Web、SDK 和 demo 默认按职责拆分模块；禁止让单个源文件持续膨胀，新增功能若使文件明显变大，必须同步拆到按功能命名的模块文件中。

**集成体验对比**：

```rust
// tikeo Rust SDK — 3 行代码集成
use tikeo_sdk::prelude::*;

#[tikeo::processor]
async fn my_task(ctx: TaskContext) -> TaskResult {
    let data = ctx.param("key")?;
    // 业务逻辑
    Ok(TaskResult::success("done"))
}

// main.rs
#[tokio::main]
async fn main() {
    tikeo::worker()
        .server("tikeo.example.com:9090")
        .app_name("my-service")
        .register(my_task)
        .start()
        .await?;
}
```

```python
# tikeo Python SDK
from tikeo import Worker, TaskContext, TaskResult

worker = Worker("tikeo.example.com:9090", app_name="my-service")

@worker.processor("my_task")
async def my_task(ctx: TaskContext) -> TaskResult:
    data = ctx.param("key")
    return TaskResult.success("done")

worker.start()
```

```go
// tikeo Go SDK
package main

import (
    tikeo "github.com/tikeo/sdk-go"
)

func main() {
    w := tikeo.NewWorker("tikeo.example.com:9090",
        tikeo.WithAppName("my-service"))

    w.Register("my_task", func(ctx tikeo.TaskContext) tikeo.TaskResult {
        return tikeo.Success("done")
    })

    w.Start()
}
```

对比 PowerJob 的集成方式——需要添加 Maven 依赖、配置 properties、实现 Java 接口、Spring Boot 启动——tikeo 的多语言 SDK 将集成成本降低到**任意语言 3-5 行代码**。

#### 4.2.3 Java Spring Boot Starter SDK

Java 端 SDK 优先支持 Spring Boot Starter 模式，目标是让现有 Spring Boot 业务以最小改造接入 tikeo Worker Tunnel。

**模块规划**：

```text
sdks/java/
├── settings.gradle.kts                  # Gradle multi-project settings
├── build.gradle.kts                     # 聚合与 group/version
├── tikeo/                               # 原生 Java 集成：gRPC client、协议模型、通用 Worker runtime
├── tikeo-spring/                        # Spring 7 集成：@TikeoProcessor 注册表与方法适配
├── tikeo-spring5/                       # Spring 5 adapter，供 Boot2 starter 使用
├── tikeo-spring6/                       # Spring 6 adapter，供 Boot3 starter 使用
├── tikeo-spring-boot2-starter/          # Spring Boot 2.7 starter
├── tikeo-spring-boot3-starter/          # Spring Boot 3.5 starter
└── tikeo-spring-boot-starter/           # Spring Boot 4.x starter
```

Java SDK Gradle 模块约束：
- `tikeo`：原生 Java 集成，包含 Worker Tunnel gRPC client、协议生成、任务上下文与结果模型。
- `tikeo-spring` / `tikeo-spring5` / `tikeo-spring6`：分别承载 Spring 7/5/6 adapter，包含 `@TikeoProcessor` 扫描、注册表和方法适配，不包含 Spring Boot autoconfigure。
- `tikeo-spring-boot-starter` / `tikeo-spring-boot2-starter` / `tikeo-spring-boot3-starter`：分别面向 Boot4/2/3，依赖匹配的 Spring adapter；每个模块必须有自己的 `src/main` / `src/test` 边界，不允许只靠 Gradle sourceSet 隐式复用形成空模块。

Java SDK 构建约束：
- 必须使用 Gradle（优先 Kotlin DSL：`settings.gradle.kts` / `build.gradle.kts`），不再使用 Maven `pom.xml` 作为主构建。
- Java toolchain 可使用当前 JDK 构建，但 SDK 源码与发布产物必须保持 `--release 17` 兼容，避免误用 Java 21+ API 破坏 Spring Boot 2/3 消费者。
- Spring Boot Starter 模式继续保留，业务侧只需依赖 starter。
- 当前 Java Core SDK 已提供真实 gRPC Worker Tunnel 客户端：注册时只发送 `client_instance_id`，读取服务端下发的权威 `worker_id`，并用于心跳、任务日志和任务结果上报；Spring Boot 2/3/4 demo 支持默认/显式 live tunnel 与结构化 namespace/app/cluster/region/worker_pool 配置。
- CI / 本地验证命令统一为 `./sdks/java/gradlew -p sdks/java test`；每个 Java SDK 子模块也必须支持 Gradle 单模块任务（如 `./sdks/java/gradlew -p sdks/java :tikeo:test`）；Maven 骨架与 `mvn -f sdks/java/pom.xml test` 文档引用不得再新增。

**业务侧使用方式**：

```java
@Component
public class BillingTasks {
    @TikeoProcessor("billing.reconcile")
    public TaskResult reconcile(TaskContext context) {
        return TaskResult.success("ok");
    }
}
```

```yaml
tikeo:
  server: https://tikeo.example.com
  app-name: billing-service
  worker-pool: prod-cn
  namespace: finance
  labels:
    region: cn
    runtime: spring-boot
```

Starter 需要提供：

> 2026-06-04 状态：Java SDK 已拆分为 Boot2/Boot3/Boot4 三套 starter 与 Spring5/Spring6 兼容 adapter，三套 Java demo 独立存在；Rust 与 Go SDK/demo 已对齐 Java demo 的结构化 namespace/app/cluster/region/clientInstanceId/processorName、Worker Tunnel live 连接、assignment token 日志上报、脚本 runner capability 与重连循环。

- `@EnableTikeoWorker` 或自动启用的 Spring Boot auto-configuration。
- `@TikeoProcessor` 注解扫描和方法适配。
- 与 Server 的 Worker Tunnel 主动连接、注册、心跳、状态上报、日志上报和日志订阅。当前已完成真实 gRPC 连接、注册、心跳、日志、任务结果回传，并已支持将 `@TikeoProcessor` 方法适配为真实任务处理器（通过 `DispatchTask.processor_name` 匹配 processor name，payload 支持 UTF-8 String / byte[] / TaskContext；空值兼容回退到 `job_id`）。
- Spring Boot lifecycle 集成：应用启动后连接，`ContextClosedEvent` 时 drain/优雅下线。
- Micrometer 指标、Actuator health indicator、结构化日志上下文。
- mTLS / token / cert rotation 配置入口。
- 默认不暴露入站端口，不要求业务 Service 被 tikeo 访问。

#### 4.2.4 动态脚本处理器设计

动态脚本是一等处理器能力，用于低频运维任务、轻量数据处理、迁移脚本、Webhook 编排和临时自动化。设计目标是**多语言可用，但默认不信任脚本**：脚本永远不在 tikeo Server 进程内执行，只能在 Worker 侧的受控执行环境中运行。

**支持语言分层**：

| 级别 | 语言/运行时 | 适用场景 | 安全策略 |
|------|-------------|----------|----------|
| 默认支持 | Shell、Python、JavaScript、TypeScript、PowerShell | 运维脚本、数据处理、API 编排 | **sandbox=auto 为默认后端选择**：可编译到 WASM/WASI 时优先 Wasmtime/WasmEdge；Python 源码若要走 WASM 需配套 Pyodide/CPython-WASI 等 runtime；原生命令/现成二进制优先 Anthropic Sandbox Runtime (srt)；JavaScript/TypeScript 优先 Deno/V8；未匹配时明确 runtime unavailable。可手动指定 wasmtime、wasmedge、srt、deno、v8、docker、podman 或 custom |
| 安全表达式 | Rhai / CEL / JSONLogic | 工作流条件、参数转换、轻量计算 | 嵌入式解释器，禁用反射、IO、网络、进程启动 |
| 直接 WASM 插件 | WASM/WASI | 可复用处理器、跨语言插件、强隔离任务 | `language=wasm` 仅作为历史/底层直接 WASM 模块兼容模式，不再出现在脚本创建/编辑语言枚举中；Wasmtime 45.x 作为 worker 侧运行时；fuel/epoch interruption + ResourceLimiter/memory cap + capability-based WASI + 签名校验；默认无网络、无预打开目录、仅允许显式 env |
| 企业扩展 | 容器化脚本运行器 | 需要系统依赖或复杂运行时的脚本 | 独立 Pod/容器，seccomp/AppArmor、NetworkPolicy、只读 rootfs |

**执行安全边界**：

1. **Server 不执行用户代码**：Server 只保存脚本定义、版本、审批状态和策略，实际执行由匹配 Worker Pool 完成。
2. **脚本版本化、发布指针与签名**：脚本内容按 content hash 存储；每次更新自动产生新版本记录（content、policy 变更均产生版本）；`scripts.released_version_id/released_version_number` 只作为软关联发布指针指向不可变 `script_versions` 快照，发布/回滚只移动指针不改历史；Worker 调度必须绑定发布快照的 bytes + SHA-256，禁止从可变 draft/current content 执行。支持任意两个版本间的 diff 对比（content diff、policy diff）；生产环境脚本必须经过审批、签名或可信发布流水线。
3. **最小权限 capability**：脚本声明所需能力，例如 `network.egress`、`fs.read:/data/input`、`secret:db-readonly`；未声明能力默认不可用。普通脚本 Worker 注册时必须声明统一脚本能力 `script`；保留 `script:<language>`、`script:*` 和 `*` 作为旧 Worker/受控池兼容能力。直接 WASM 模块仍要求 `script:wasm`。具体语言与沙箱后端由 Worker 在收到 `ScriptProcessorBinding` 后根据 `language` 与 `sandbox.backend` 自适应选择。能力不等同于后端类型：脚本 policy 的 `sandbox.backend` 默认为 `auto`，也可手动指定 `wasmtime`、`wasmedge`、`srt`、`deno`、`v8`、`docker`、`podman` 或 `custom`；仅受控 Worker Pool 可使用 `script:*` 或 `*` 作为显式兜底能力。
4. **资源限制**：每次执行强制 timeout、CPU quota、内存上限、输出大小、日志速率、最大并发和重试预算。
5. **文件系统隔离**：默认临时工作目录；只读挂载输入；输出通过受控 artifact API 写入；禁止访问宿主敏感路径。
6. **网络隔离**：默认禁止出站网络；允许时必须经过 URL policy、DNS pinning、内网/metadata 地址阻断、TLS 校验和请求审计。
7. **凭证隔离**：脚本只能通过 Secret reference 获取临时凭证；日志和错误栈自动脱敏；禁止把密钥作为普通参数明文存储。
8. **危险能力审批**：启用网络、写文件、执行外部命令、访问 Secret、长超时、高资源配额等能力需要策略审批。
9. **审计与可追溯**：记录脚本版本、提交人、审批人、执行 Worker、输入摘要、能力清单、资源用量、网络目标和 artifact hash；所有脚本更新必须保留历史版本并支持 diff 对比。
10. **执行治理失败分类**：调度与 Worker 结果必须把脚本失败归类为可观测治理事件，至少覆盖无匹配 Worker capability、Worker 未注册 runner、策略拒绝、内容摘要不匹配、超时、输出超限、运行时不可用。当前实现通过实例日志写入 `script_execution_governance` JSON，包含 `failure_class` 和 message，便于后续审计/告警聚合。

**推荐执行路径**：

```text
Job Definition
  -> Script Processor(language, code_ref, runtime_policy)
  -> Tikeo 选择具备统一脚本 capability 的 Worker Pool（优先 `script`，兼容 `script:<language>` / `script:wasm` / `script:*`）
  -> Server 将 released script_version 快照 bytes + SHA-256 + version metadata 绑定到 Worker Tunnel 任务
  -> Worker 校验签名/hash 后选择显式注册的 Runner
  -> 默认 Runner 使用 `sandbox=auto` 自适应：WASM 编译路径优先 Wasmtime/WasmEdge，Python 源码走 WASM 时必须配套 Pyodide/CPython-WASI runtime，原生命令/二进制优先 srt，JavaScript/TypeScript 优先 Deno/V8，未匹配则明确 runtime unavailable；可按策略显式选择 wasmtime/wasmedge/srt/deno/v8/docker/podman/custom
  -> 执行脚本并流式上报日志/指标/artifact
  -> 清理临时目录并提交审计事件
```

**禁止项**：

- 禁止在 Server 进程中嵌入 Groovy/Python/Node 等解释器执行用户脚本。
- 禁止默认继承 Worker 进程环境变量、宿主网络和宿主文件系统。
- 禁止脚本直接读取平台数据库或内部管理 API；必须通过受控 Service Account 与 RBAC 授权。
- 禁止把“动态脚本”作为绕过正式 SDK、权限和审计的后门。



### 4.3 tikeo CLI

```bash
# 服务管理
tikeo serve                           # 启动 server（单机模式，SQLite）
tikeo serve --cluster --db postgres   # 启动 server（集群模式）

# 任务管理
tikeo job create --file job.yaml      # 从 YAML 创建任务
tikeo job list --app my-service       # 列出任务
tikeo job trigger <job-id>            # 手动触发
tikeo job cancel <instance-id>        # 取消执行

# 工作流管理
tikeo workflow create --file flow.yaml
tikeo workflow trigger <wf-id>
tikeo workflow visualize <wf-id>      # 终端 ASCII 可视化

# 集群管理
tikeo cluster status                  # 集群健康状态
tikeo cluster workers                 # Worker 列表
tikeo cluster reschedule --app xxx    # 强制重新调度

# 数据管理
tikeo migrate                         # 执行数据库迁移
tikeo export --format json            # 导出任务定义
tikeo import --file backup.json       # 导入任务定义
```

---

## 5. 通信协议设计

### 5.1 协议选型：gRPC (HTTP/2)

xxl-job 与 PowerJob 的通信问题不是“协议实现细节”，而是公共服务化能力的根本边界：

- xxl-job：Admin 通过 HTTP 反向调用 Executor 内嵌 Netty 服务，Executor 必须注册可访问地址。
- PowerJob：Server 通过 AKKA/HTTP/MU 反向调用 Worker，上报地址还要区分 bind address 与 external address。
- tikeo：Worker 主动建立 gRPC/HTTP2 长连接隧道，Server 不要求直连 Worker Pod，也不依赖 Worker 可被公网、跨 namespace 或跨集群访问。

| 场景 | xxl-job | PowerJob | tikeo |
|------|---------|----------|-----------|
| Server/Admin → Worker/Executor 分发 | HTTP 调 Executor `/run` | Akka TCP / HTTP / MU 调 Worker | gRPC 双向流下发 `DispatchTask` |
| Worker/Executor → Server 心跳 | Executor 注册/心跳到 Admin | Akka/HTTP/MU 心跳上报 | gRPC stream heartbeat，连接即租约 |
| 日志传输 | Admin 轮询 Executor 本地日志 | Worker 批量上报，队列满或 Server 不可用可能丢 | gRPC Client Stream，背压 + 可选 WAL |
| 状态上报 | HTTP callback | Akka/HTTP/MU 上报 | gRPC stream/unary，attempt token 幂等 |
| Server 集群协调 | DB `FOR UPDATE` 全局锁 | DB lock + currentServer + PING | Raft / lease shard / fencing token |
| K8s/Docker/NAT/多级网关适配 | 需要 Executor 入站可达 | 需要 Worker 入站可达，多协议多端口 | 只需要 Worker 出站访问 tikeo tunnel endpoint，支持跨 namespace/cluster/VPC |
| 管理 API | HTTP MVC | HTTP REST | REST + gRPC Gateway + gRPC reflection |

因此 tikeo 使用**单一 gRPC 协议**解决 Worker 通信、任务分发、日志流、状态上报和集群内部 RPC；REST 只作为管理面 API，不进入核心执行链路。

### 5.2 Protobuf 协议定义

```protobuf
// === Worker 协议 ===

service WorkerService {
  rpc DispatchTask(DispatchRequest) returns (DispatchResponse);
  rpc CancelTask(CancelRequest) returns (CancelResponse);
  rpc HealthCheck(HealthCheckRequest) returns (HealthCheckResponse);
}

service ServerService {
  rpc Heartbeat(stream HeartbeatRequest) returns (stream HeartbeatResponse);
  rpc ReportStatus(StatusReport) returns (StatusAck);
  rpc StreamLogs(stream LogEntry) returns (LogAck);
  rpc ReportSubTask(SubTaskReport) returns (SubTaskAck);
}

// === API 协议 ===

service JobService {
  rpc CreateJob(CreateJobRequest) returns (Job);
  rpc UpdateJob(UpdateJobRequest) returns (Job);
  rpc DeleteJob(DeleteJobRequest) returns (google.protobuf.Empty);
  rpc GetJob(GetJobRequest) returns (Job);
  rpc ListJobs(ListJobsRequest) returns (ListJobsResponse);
  rpc TriggerJob(TriggerRequest) returns (Instance);
  rpc CancelInstance(CancelInstanceRequest) returns (google.protobuf.Empty);
  rpc GetInstance(GetInstanceRequest) returns (Instance);
  rpc StreamInstanceLog(LogStreamRequest) returns (stream LogEntry);
}

service WorkflowService {
  rpc CreateWorkflow(CreateWorkflowRequest) returns (Workflow);
  rpc UpdateWorkflow(UpdateWorkflowRequest) returns (Workflow);
  rpc DeleteWorkflow(DeleteWorkflowRequest) returns (google.protobuf.Empty);
  rpc TriggerWorkflow(TriggerWorkflowRequest) returns (WorkflowInstance);
  rpc GetWorkflowInstance(GetWorkflowInstanceRequest) returns (WorkflowInstance);
  rpc StreamWorkflowStatus(StreamStatusRequest) returns (stream WorkflowNodeStatus);
}

service AdminService {
  rpc ListWorkers(ListWorkersRequest) returns (ListWorkersResponse);
  rpc DrainWorker(DrainWorkerRequest) returns (google.protobuf.Empty);
  rpc ListTenants(ListTenantsRequest) returns (ListTenantsResponse);
  rpc UpsertSecret(UpsertSecretRequest) returns (SecretRef);
  rpc QueryAuditLog(QueryAuditLogRequest) returns (QueryAuditLogResponse);
}

service UiRealtimeService {
  rpc WatchDashboard(WatchDashboardRequest) returns (stream DashboardEvent);
  rpc WatchInstance(WatchInstanceRequest) returns (stream InstanceEvent);
}
```

### 5.3 端口模型

PowerJob 需要 4 个端口 (7700, 10086, 10010, 10077)。tikeo 只需**1 个端口**：

```
tikeo-server:9090
├── gRPC (h2)           — Worker Tunnel + API + 集群 RPC
├── HTTP/1.1 (REST)     — Web 控制台 + REST API
└── WebSocket           — 浏览器实时日志（可选，gRPC-Web 亦可）
```

### 5.4 Worker 主动连接模型

tikeo 的 Worker Tunnel 是对 xxl-job / PowerJob 反向调用模型的直接修正，也是跨容器、跨 namespace、跨集群部署的核心能力。Worker 注册、心跳、任务分发、取消、日志、证书轮换和配置下发都复用同一条由 Worker 主动发起的长连接。

```protobuf
service WorkerTunnelService {
  rpc OpenTunnel(stream WorkerMessage) returns (stream ServerMessage);
}
```

消息类型：

| 方向 | 消息 |
|------|------|
| Worker → Server | Register、Heartbeat、TaskStatus、TaskResult、LogChunk、Metrics、LeaseRenew |
| Server → Worker | DispatchTask、CancelTask、Drain、UpdateConfig、RotateCert、Ping |

该模型带来的直接收益：

1. **无业务入站端口**：业务 Pod/容器不需要为调度暴露 Service、Ingress、NodePort 或公网端口。
2. **穿透多级网络层级**：只要求 Worker 能出站访问 tikeo tunnel endpoint；中间可以是 Docker bridge、K8s Service、Ingress、API Gateway、Service Mesh、NAT Gateway、VPN、专线或跨云负载均衡。
3. **反向调用走既有通道**：Server 对 Worker 的 DispatchTask、CancelTask、Drain、RotateCert 等“反向调用”不是新建到 Worker 的连接，而是写回 Worker 已建立的双向流。
4. **跨 namespace/cluster/VPC 简化**：Worker 所在网络无需被 tikeo 路由可达；注册时上报 app、namespace、cluster、region、tenant、capabilities 和 labels，Server 按逻辑属性寻址。
5. **连接即租约**：连接断开后，Server 可基于 lease timeout 重新调度；Worker 重连后按 instance/attempt token 幂等恢复。
6. **日志天然流式**：日志、状态、取消指令都在同一安全通道中传输。
7. **安全边界更小**：mTLS、证书轮换、限流、审计集中在单协议网关。

**网络穿透要求**：

- Worker Tunnel 必须支持 HTTP/2 keepalive、断线重连、指数退避、会话恢复、连接迁移和多 endpoint failover。
- 必须兼容 K8s Ingress/Gateway API、Nginx/Envoy/Traefik、云负载均衡、Service Mesh sidecar、Docker bridge 和企业代理。
- 对不稳定链路，Worker 本地可选 WAL 缓冲状态、日志和结果，上线后按 sequence id 补传。
- Server 侧不保存 Worker IP:Port 作为调用地址，只保存逻辑身份、连接 ID、租约、能力与最近心跳。
- 所有 Server→Worker 指令必须通过连接路由表投递；若连接不存在，任务进入等待/重调度/失败策略，而不是尝试直连 Worker。



### 5.5 HTTP 接口模块设计

HTTP 接口是平台管理面的一等能力，面向 Web UI、CLI、CI/CD、GitOps、外部平台集成和运维自动化。核心执行链路仍以 gRPC/Worker Tunnel 为准；HTTP API 不直接回连 Worker，也不绕过调度状态机。

**HTTP 模块边界**：

| 模块 | 职责 | 说明 |
|------|------|------|
| `gateway/http.rs` | Axum 路由、middleware、错误映射 | REST API 入口 |
| `gateway/openapi.rs` | OpenAPI 3.1 文档生成 | 支持 OpenAPI JSON / SDK 生成；不提供浏览器文档 UI |
| `gateway/realtime.rs` | SSE/WebSocket/gRPC-Web | 实时日志、状态、Dashboard 事件 |
| `api/dto/` | HTTP DTO 与分页模型 | 与内部领域模型隔离 |
| `api/handler/` | Job/Workflow/Worker/Auth/Audit 等 handler | 只编排 application service |
| `api/middleware/` | AuthN/AuthZ、租户解析、审计、限流、幂等 | 所有写操作必须审计 |
| `web/` | React + Ant Design 嵌入式 SPA 静态资源 | 使用 Bun 构建，由 server 单二进制托管 |

**REST API 规范**：

- API 前缀：`/api/v1`；管理控制台页面走 `/`，静态资源走 `/assets/*`。
- 认证：支持 Session Cookie、Bearer Token、OIDC、Service Account Token。
- 授权：所有接口按 `tenant / namespace / app / worker_pool / resource / action` 做 RBAC。
- 幂等：创建、触发、取消、审批等写操作支持 `Idempotency-Key`。
- 并发控制：更新接口支持 `resource_version` / `If-Match`，避免覆盖并发变更。
- 分页：统一 `page_size`、`page_token`、`sort`、`filter`。
- 响应体：所有 HTTP 业务接口统一返回 `{code, message, data}`；`code` 为 int，`0` 表示成功，非 `0` 表示失败；`message` 为响应信息；`data` 必须显式存在，允许为 `null`。
- 错误：错误响应同样使用 `{code, message, data}`，其中 `code != 0`，错误细节可放入 `data`。
- 审计：所有写操作、密钥读取、脚本发布、权限变更、手动触发和取消都写审计日志。
- OpenAPI：启动后暴露 `/api-docs/openapi.json`；不提供文档 UI，CI 中校验 OpenAPI 兼容性。

**核心 HTTP 路由规划**：

| 资源 | 方法与路径 | 用途 |
|------|------------|------|
| Auth | `POST /api/v1/auth/login`、`POST /api/v1/auth/logout`、`GET /api/v1/auth/me` | 登录、退出、当前身份 |
| Tenants | `GET/POST /api/v1/tenants`、`GET/PATCH/DELETE /api/v1/tenants/{tenant}` | 租户管理 |
| Namespaces | `GET/POST /api/v1/namespaces`、`PATCH /api/v1/namespaces/{namespace}` | namespace 管理 |
| Apps | `GET/POST /api/v1/apps`、`GET/PATCH/DELETE /api/v1/apps/{app}` | 应用管理 |
| Jobs | `GET/POST /api/v1/jobs`、`GET/PATCH/DELETE /api/v1/jobs/{job}` | 任务 CRUD |
| Job Actions | `POST /api/v1/jobs/{job}:trigger`、`:pause`、`:resume`、`:validate`、`:simulate` | 触发、暂停、恢复、校验、调度仿真 |
| Instances | `GET /api/v1/instances`、`GET /api/v1/instances/{instance}`、`POST /api/v1/instances/{instance}:cancel`、`:retry` | 实例查询、取消、重试 |
| Logs | `GET /api/v1/instances/{instance}/logs`、`GET /api/v1/instances/{instance}/logs:stream` | 历史日志与实时日志 |
| Workflows | `GET/POST /api/v1/workflows`、`GET/PATCH/DELETE /api/v1/workflows/{workflow}` | 工作流定义管理 |
| Workflow Actions | `POST /api/v1/workflows/{workflow}:trigger`、`:validate`、`:dry-run` | 工作流触发、校验、试运行 |
| Workers | `GET /api/v1/workers`、`GET /api/v1/workers/{worker}`、`POST /api/v1/workers/{worker}:drain` | Worker 观测与摘流 |
| Worker Pools | `GET/POST /api/v1/worker-pools`、`PATCH /api/v1/worker-pools/{pool}` | Worker Pool 管理 |
| Scripts | `GET/POST /api/v1/scripts`、`POST /api/v1/scripts/{script}/publish`、`POST /api/v1/scripts/{script}/rollback`、`GET /api/v1/scripts/{script}/versions`、`GET /api/v1/scripts/{script}/diff?v1=&v2=` | 动态脚本版本、发布指针、回滚、版本历史与 diff 对比；所有响应保持 `{code,message,data}` |
| Secrets | `GET/POST /api/v1/secrets`、`POST /api/v1/secrets/{secret}:rotate` | Secret reference 管理 |
| Alerts | `GET/POST /api/v1/alert-rules`、`GET/POST /api/v1/notification-channels` | 告警规则与通知渠道 |
| Audit | `GET /api/v1/audit-logs`, `GET /api/v1/audit-logs:export?format=json` | 审计查询与受治理 JSON 导出（500 行上限、`audit:read` 权限、标准 envelope） |
| Metrics | `GET /metrics`、`GET /api/v1/metrics/summary` | Prometheus 与控制台摘要 |
| System | `GET /healthz`、`GET /readyz`、`GET /api/v1/cluster` | 健康检查与集群状态 |

**实时接口**：

| 场景 | 推荐协议 | 路径 |
|------|----------|------|
| 实例日志跟随 | SSE / WebSocket | `/api/v1/instances/{instance}/logs:stream` |
| 实例状态变化 | SSE / WebSocket | `/api/v1/instances/{instance}/events` |
| Dashboard 指标刷新 | SSE | `/api/v1/dashboard/events` |
| 工作流节点状态 | SSE / WebSocket | `/api/v1/workflow-instances/{id}/events` |

### 5.6 Web UI 管理控制台设计

Web UI 是平台的默认管理入口，必须覆盖日常开发、运维、排障、审计和治理场景。前端独立放在 `./web/`，以嵌入式 SPA 随 server 单二进制发布，同时允许独立静态托管以适配企业网关。

**信息架构**：

| 一级模块 | 页面 | 核心能力 |
|----------|------|----------|
| Dashboard | 概览、调度趋势、失败率、延迟、Worker 在线率 | 平台健康态势、近期异常、SLA/SLO 摘要 |
| Jobs | 任务列表、任务详情、创建/编辑、版本历史、调度仿真 | CRON/FIX_RATE/FIX_DELAY/API/延迟/一次性任务管理 |
| Instances | 实例列表、实例详情、attempt 详情、日志、重试/取消 | 运行态排障与操作 |
| Workflows | DAG 列表、可视化编辑器、YAML/JSON 双模式、试运行 | 工作流编排、diff、回放、人工节点 |
| Workers | Worker 列表、Worker Pool、连接详情、能力标签、摘流 | 观测跨集群 Worker、连接租约、运行时能力 |
| Scripts | 脚本列表、在线编辑、版本历史、版本 diff 对比、审批、沙箱策略、执行记录 | 多语言动态脚本治理 |
| Apps & Tenants | 租户、namespace、app、quota、标签 | 多租户与资源边界 |
| Secrets | Secret reference、轮换、使用关系 | 凭证治理，不展示明文 |
| Alerts | 告警规则、通知渠道、静默、升级策略 | 运维告警管理 |
| Audit | 审计日志、过滤、导出、SIEM 链接 | 合规追踪 |
| Settings | OIDC、RBAC、API Token、系统配置、集群节点 | 平台配置 |

**关键交互要求**：

1. **任务创建向导**：选择调度类型、执行器类型、Worker Pool、参数 schema、重试/超时、告警和权限策略。
2. **调度仿真**：创建或修改任务前展示未来 N 次触发时间、misfire 行为、时区/DST 影响和资源预估。
3. **实例排障视图**：时间线展示 queued、dispatched、running、heartbeat、log、retry、cancel、finished 等事件。
4. **实时日志**：支持 follow、关键字过滤、日志级别过滤、下载、对象存储归档链接和敏感字段脱敏提示。
5. **工作流可视化**：DAG 拖拽编辑、节点状态着色、上下文查看、失败节点重跑、从节点恢复。
6. **脚本安全编辑**：语言选择、依赖声明、capability 声明、沙箱策略预览、dry-run、审批流和版本 diff。
7. **Worker 连接可观测**：展示 worker id、app、pool、cluster、region、labels、capabilities、连接时长、最后心跳、当前任务、drain 状态。
8. **权限感知 UI**：根据 RBAC 隐藏或禁用操作；危险操作二次确认并展示影响范围。
9. **GitOps 友好**：每个 Job/Workflow/Alert 支持 YAML/JSON 查看、复制、导入、导出和 diff。
10. **可访问性与国际化**：支持暗色模式、键盘操作、基础 a11y、中文/英文文案资源。

**前端工程约束**：

- 固定使用 React + TypeScript + Vite + Ant Design + Bun，依赖尽量使用当前最新稳定版；最终产物由 `include_dir` 或等价机制嵌入 server。
- API client 从 OpenAPI 生成，避免手写漂移。
- 表单 schema 优先从后端 JSON Schema/OpenAPI 元数据生成。
- 实时能力优先 SSE，复杂双向交互可用 WebSocket；浏览器不直接访问 Worker。
- UI 不存储长期凭证；Token 刷新、CSRF、防 XSS、CSP 与安全响应头由 gateway 统一处理。

---

## 6. 核心时序图

### 6.1 任务调度完整生命周期

```mermaid
sequenceDiagram
    participant CRON as Tikeo<br/>(CRON Engine)
    participant DB as Database
    participant DQ as Trigger Queue
    participant DSP as Dispatcher
    participant W as Worker
    participant LOG as Log Streamer
    participant ALERT as Alert Service

    CRON->>DB: 扫描到期 Job
    DB-->>CRON: 返回 Job 列表

    loop 每个 Job
        CRON->>DB: INSERT instance (WAITING)
        CRON->>DQ: 发送 Trigger
    end

    DQ->>DSP: 消费 Trigger
    DSP->>DB: 查询可用 Worker
    DB-->>DSP: Worker 列表

    DSP->>DSP: 选择目标 Worker<br/>(Random/RR/Tag)
    DSP->>DB: UPDATE instance (DISPATCHED)
    DSP->>W: gRPC DispatchTask

    W-->>DSP: gRPC ACK
    W->>DB: UPDATE instance (RUNNING)

    loop 任务执行中
        W->>LOG: gRPC StreamLogs<br/>(Client Stream, 背压)
        LOG->>DB: 批量写入日志
    end

    alt 执行成功
        W->>DB: UPDATE instance (SUCCESS)
        W->>DSP: ReportStatus(SUCCESS)
    else 执行失败 & 可重试
        W->>DB: UPDATE instance (FAILED)
        W->>DSP: ReportStatus(FAILED, retryable)
        DSP->>CRON: 重新入队 (retry_count++)
    else 执行失败 & 不可重试
        W->>DB: UPDATE instance (FAILED)
        W->>DSP: ReportStatus(FAILED)
        DSP->>ALERT: 触发告警
    end
```

### 6.2 Worker 注册与心跳

```mermaid
sequenceDiagram
    participant W as Worker
    participant GW as gRPC Gateway
    participant AUTH as Auth Service
    participant DB as Database
    participant RAFT as Raft Leader

    rect rgb(240, 248, 255)
        Note over W,DB: 注册阶段
        W->>GW: gRPC Connect (app_key, app_secret)
        GW->>AUTH: 验证 HMAC 签名
        AUTH-->>GW: 验证通过

        GW->>RAFT: 转发注册请求
        RAFT->>DB: INSERT workers<br/>(worker_id, address, tags)
        DB-->>RAFT: OK
        RAFT-->>GW: 注册成功
        GW-->>W: Connect ACK + session_token
    end

    rect rgb(255, 248, 240)
        Note over W,DB: 心跳阶段 (每 10s)
        loop 每 10 秒
            W->>GW: Heartbeat<br/>(system_metrics, active_tasks)
            GW->>DB: UPDATE workers<br/>SET last_heartbeat = NOW()

            alt 心跳超时 (30s)
                GW->>GW: 标记 Worker OFFLINE
                GW->>RAFT: 触发任务重调度
                RAFT->>DB: 更新受影响 instance<br/>为 WAITING
            end
        end
    end

    rect rgb(248, 255, 240)
        Note over W,DB: 优雅下线
        W->>GW: Heartbeat(DRAINING)
        GW->>DB: UPDATE workers SET status = DRAINING
        Note over GW: 不再分发新任务<br/>等待现有任务完成
        W->>GW: Heartbeat(OFFLINE)
        GW->>DB: DELETE workers
    end
```

### 6.3 MapReduce 分布式执行

```mermaid
sequenceDiagram
    participant SCH as Tikeo
    participant MASTER as Master Worker<br/>(TaskTracker)
    participant W1 as Worker 1
    participant W2 as Worker 2
    participant W3 as Worker 3
    participant DB as Database

    SCH->>MASTER: DispatchTask (MapReduce)

    rect rgb(240, 248, 255)
        Note over MASTER,W3: Map 阶段
        MASTER->>MASTER: 生成子任务列表<br/>(MapProcessor.process)

        par 并行分发子任务
            MASTER->>W1: DispatchSubTask(sub_1)
            MASTER->>W2: DispatchSubTask(sub_2)
            MASTER->>W3: DispatchSubTask(sub_3)
        and
            MASTER->>DB: UPDATE instance<br/>subtask_count = 3
        end

        W1-->>MASTER: SubTaskResult(sub_1, ok)
        W2-->>MASTER: SubTaskResult(sub_2, ok)
        W3-->>MASTER: SubTaskResult(sub_3, ok)
    end

    rect rgb(248, 255, 240)
        Note over MASTER: Reduce 阶段
        MASTER->>MASTER: ReduceProcessor.reduce<br/>[sub_1, sub_2, sub_3]
        MASTER->>DB: UPDATE instance (SUCCESS)<br/>result = reduce_output
        MASTER->>SCH: ReportStatus(SUCCESS)
    end

    Note over SCH,DB: 内存不足时<br/>子任务结果自动换出到磁盘 (SWAP)
```

### 6.4 Server 集群 Raft 选举

```mermaid
sequenceDiagram
    participant S1 as Server 1
    participant S2 as Server 2
    participant S3 as Server 3
    participant DB as Shared Database

    Note over S1,S3: 初始状态: S1=Leader, S2/S3=Follower

    rect rgb(255, 230, 230)
        Note over S1,S3: Leader 故障场景
        S1-xS1: 进程崩溃 / 网络隔离
        Note over S2,S3: 心跳超时 (election timeout)
    end

    rect rgb(255, 248, 230)
        Note over S2,S3: 选举阶段
        S2->>S3: RequestVote(term=2)
        S3->>S2: RequestVote(term=2, granted)
        Note over S2: 获得多数票
        S2->>S2: 成为新 Leader (term=2)
    end

    rect rgb(240, 255, 240)
        Note over S2,S3: 新 Leader 交接
        S2->>DB: 更新 leader 记录
        S2->>S3: AppendEntries(heartbeat)

        Note over S2,DB: 接管所有 app 调度
        S2->>DB: 扫描 WAITING 实例
        DB-->>S2: 待调度任务列表
        S2->>S2: 重新分发任务到 Worker
    end

    rect rgb(240, 240, 255)
        Note over S1,S3: 旧 Leader 恢复
        S1->>S2: AppendEntries(term=1)
        S2-->>S1: AppendEntries(term=2, rejected)
        Note over S1: 发现更高 term<br/>自动降级为 Follower
        S1->>S2: AppendEntries(term=2)
        S2-->>S1: ACK
    end
```

### 6.5 工作流 DAG 执行时序

```mermaid
sequenceDiagram
    participant API as REST/gRPC API
    participant WF as Workflow Engine
    participant SCH as Tikeo
    participant W_A as Worker (任务 A)
    participant W_B as Worker (任务 B)
    participant W_C as Worker (任务 C)
    participant W_D as Worker (任务 D)

    API->>WF: TriggerWorkflow(wf_id)

    WF->>WF: 拓扑排序 DAG<br/>A → [B 或 C] → D

    rect rgb(240, 248, 255)
        Note over WF,W_A: Layer 0: 节点 A
        WF->>SCH: Dispatch Job A
        SCH->>W_A: gRPC DispatchTask
        W_A-->>WF: Result A (SUCCESS)<br/>context: {status: "ok"}
    end

    rect rgb(255, 248, 240)
        Note over WF,W_C: Layer 1: 条件分支
        WF->>WF: 评估条件<br/>context.status == "ok" → B<br/>context.status == "error" → C

        WF->>SCH: Dispatch Job B (条件命中)
        SCH->>W_B: gRPC DispatchTask
        W_B-->>WF: Result B (SUCCESS)
    end

    rect rgb(248, 255, 240)
        Note over WF,W_D: Layer 2: 节点 D
        WF->>WF: 合并上下文<br/>{A: result_a, B: result_b}

        WF->>SCH: Dispatch Job D
        SCH->>W_D: gRPC DispatchTask
        W_D-->>WF: Result D (SUCCESS)
    end

    WF->>WF: Workflow Complete
    WF->>API: 通知完成
```

---

## 7. 数据模型与存储层

### 7.1 ORM 选型：SeaORM

> **为什么不用 toasty？** tokio-rs/toasty 目前处于 Preview 阶段，缺少事务、聚合查询、迁移系统、编译时检查等生产必备特性，不适合作为任务调度平台的存储层。

**SeaORM 选型理由**：

| 特性 | SeaORM | toasty | SQLx (裸 SQL) | Diesel |
|------|--------|--------|---------------|--------|
| 异步原生 | ✅ (基于 SQLx) | ✅ | ✅ | ❌ (需 diesel-async) |
| 多数据库 | ✅ SQLite/MySQL/Pg | Preview | ✅ | ✅ |
| 迁移系统 | ✅ | ❌ | ✅ | ✅ |
| 事务支持 | ✅ | ❌ | ✅ | ✅ |
| 聚合查询 | ✅ | ❌ | ✅ | ✅ |
| 关联关系 | ✅ | ✅ | ❌ 手动 | ✅ |
| 编译时检查 | 部分 (schema) | ❌ | ✅ (SQL) | ✅ (schema+query) |
| 生产就绪 | ✅ | ❌ | ✅ | ✅ |
| 查询风格 | Builder Pattern | Macro | Raw SQL | DSL |

**SeaORM 代码示例**：

```rust
use sea_orm::entity::prelude::*;
use sea_orm::{Database, QueryOrder, QueryFilter, Set};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "jobs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub app_id: i64,
    pub name: String,
    pub schedule_type: String,
    pub schedule_expr: Option<String>,
    pub execute_type: String,
    pub processor_type: String,
    pub enabled: bool,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

// 查询示例：获取待调度的任务
let jobs = Jobs::find()
    .filter(Column::Enabled.eq(true))
    .filter(Column::ScheduleType.ne("API"))
    .order_by_asc(Column::Id)
    .all(&db)
    .await?;

// 事务示例：原子创建实例并更新状态
let instance = db.transaction::<_, (), DbErr>(|txn| {
    Box::pin(async move {
        let inst = instances::ActiveModel {
            job_id: Set(job.id),
            status: Set("WAITING".to_string()),
            ..Default::default()
        };
        let inst = inst.insert(txn).await?;

        // 更新 Job 最后触发时间
        let mut job: jobs::ActiveModel = job.into();
        job.last_trigger_at = Set(Some(chrono::Utc::now()));
        job.update(txn).await?;

        Ok(())
    })
}).await?;
```

### 7.2 支持的数据库矩阵

| 数据库 | 场景 | 状态 | 说明 |
|--------|------|------|------|
| **SQLite** | 单机/开发/嵌入式 | ✅ 必须支持 | 零配置，单文件部署，`./tikeo serve` 直接可用 |
| **MySQL 8.x** | 中小规模生产 | ✅ 必须支持 | PowerJob 用户迁移首选 |
| **PostgreSQL 15+** | 大规模生产/集群 | ✅ 必须支持 | 高并发、JSONB、Citus 水平扩展 |
| **CockroachDB** | 地理分布/云原生 | ✅ 必须支持 | 分布式 SQL，Serverless 友好 |
> Phase 2 implementation note: `tikeo-storage` now enables `sqlx-postgres` alongside SQLite/MySQL. PostgreSQL and CockroachDB use `postgres://` URLs; CockroachDB relies on PostgreSQL wire protocol compatibility. Database relationships remain soft-linked by id fields only; no foreign keys are introduced for any backend.

> Phase 4 hardening note (2026-06-05): SQLite legacy/dev schema compatibility is no longer an untracked post-migrate hook. The compatibility upgrade lives in the explicit SeaORM migration `crates/tikeo-storage/src/migration/sqlite_compat.rs`, is recorded in `seaql_migrations` as `sqlite_compat`, remains idempotent for existing dev databases, and keeps the no-foreign-key soft-link rule. New schema changes must be added as explicit migration entries or clearly scoped helper modules, not hidden startup patches.

| **MariaDB** | MySQL 兼容替代 | 🔄 兼容支持 | 通过 MySQL driver 兼容 |

### 7.3 存储抽象层设计

```mermaid
flowchart TD
    subgraph App["业务层"]
        SCH["Tikeo"]
        WF["Workflow Engine"]
        AUTH["Auth Module"]
    end

    subgraph Repo["Repository 抽象层<br/>(tikeo 自定义)"]
        JOB_REPO["JobRepository<br/>CRUD + 查询"]
        INST_REPO["InstanceRepository<br/>状态机 + 聚合"]
        WF_REPO["WorkflowRepository<br/>DAG 存储"]
        USER_REPO["UserRepository<br/>RBAC"]
    end

    subgraph ORM["SeaORM"]
        ENT["Entity 定义<br/>(DeriveEntityModel)"]
        MIG["Migration Engine<br/>(版本化 Schema)"]
    end

    subgraph Drivers["SQLx 驱动"]
        SQLITE["rusqlite<br/>SQLite"]
        MYSQL["mysql_async<br/>MySQL / MariaDB"]
        PG["sqlx-postgres<br/>PostgreSQL"]
        CRDB["sqlx-postgres<br/>CockroachDB"]
    end

    App --> Repo
    Repo --> ORM
    ORM --> Drivers

    SQLITE --> DB1[(file.db)]
    MYSQL --> DB2[(MySQL 8.x)]
    PG --> DB3[(PostgreSQL)]
    CRDB --> DB4[(CockroachDB)]
```

### 7.4 核心表结构

> 以下 DDL 以 PostgreSQL 语法为基准。SeaORM Migration 负责跨数据库适配（类型映射、索引语法差异等）。

```sql
-- 命名空间
CREATE TABLE namespaces (
    id          BIGINT PRIMARY KEY,
    name        VARCHAR(64) NOT NULL UNIQUE,
    description VARCHAR(512),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 应用
CREATE TABLE apps (
    id              BIGINT PRIMARY KEY,
    namespace_id    BIGINT NOT NULL, -- soft link: namespaces.id
    name            VARCHAR(128) NOT NULL,
    app_key         VARCHAR(64) NOT NULL UNIQUE,
    app_secret      VARCHAR(128) NOT NULL,
    description     VARCHAR(512),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(namespace_id, name)
);

-- 任务定义
CREATE TABLE jobs (
    id              BIGINT PRIMARY KEY,
    app_id          BIGINT NOT NULL, -- soft link: apps.id
    name            VARCHAR(256) NOT NULL,
    description     TEXT,

    schedule_type   VARCHAR(16) NOT NULL,
    schedule_expr   VARCHAR(256),
    timezone        VARCHAR(64) DEFAULT 'UTC',

    execute_type    VARCHAR(16) NOT NULL,
    processor_type  VARCHAR(32) NOT NULL,
    processor_info  JSONB NOT NULL,
    task_params     JSONB DEFAULT '{}',

    max_instance_num    INT DEFAULT 1,
    instance_timeout_ms BIGINT DEFAULT 0,
    max_retry_count     INT DEFAULT 0,
    retry_interval_ms   BIGINT DEFAULT 0,

    memory_limit_mb     INT DEFAULT 0,
    cpu_limit           DECIMAL(4,2) DEFAULT 0,

    queue_capacity      INT DEFAULT 1000,
    queue_policy        VARCHAR(16) DEFAULT 'DROP',

    enabled             BOOLEAN DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(app_id, name)
);

-- 工作流定义
CREATE TABLE workflows (
    id              BIGINT PRIMARY KEY,
    app_id          BIGINT NOT NULL, -- soft link: apps.id
    name            VARCHAR(256) NOT NULL,
    description     TEXT,
    dag_def         JSONB NOT NULL,
    context_schema  JSONB,
    max_instance_num INT DEFAULT 1,
    enabled          BOOLEAN DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(app_id, name)
);

-- 任务实例
CREATE TABLE instances (
    id              BIGINT PRIMARY KEY,
    job_id          BIGINT NOT NULL, -- soft link: jobs.id
    app_id          BIGINT NOT NULL, -- soft link: apps.id
    instance_number BIGINT NOT NULL,

    status          VARCHAR(16) NOT NULL DEFAULT 'WAITING',
    result          TEXT,
    error_message   TEXT,

    triggered_at    TIMESTAMPTZ,
    dispatched_at   TIMESTAMPTZ,
    started_at      TIMESTAMPTZ,
    finished_at     TIMESTAMPTZ,

    retry_count     INT DEFAULT 0,
    worker_address  VARCHAR(256),

    workflow_instance_id BIGINT,
    workflow_node_id     BIGINT,

    outer_key       VARCHAR(256),
    trace_id        VARCHAR(128),

    task_params     JSONB,
    reported_metrics JSONB,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 工作流实例
CREATE TABLE workflow_instances (
    id              BIGINT PRIMARY KEY,
    workflow_id     BIGINT NOT NULL, -- soft link: workflows.id
    app_id          BIGINT NOT NULL, -- soft link: apps.id

    status          VARCHAR(16) NOT NULL DEFAULT 'RUNNING',
    context         JSONB DEFAULT '{}',

    triggered_at    TIMESTAMPTZ,
    finished_at     TIMESTAMPTZ,

    outer_key       VARCHAR(256),
    trace_id        VARCHAR(128),

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Worker 注册信息
CREATE TABLE workers (
    id              BIGINT PRIMARY KEY,
    app_id          BIGINT NOT NULL, -- soft link: apps.id
    worker_id       VARCHAR(128) NOT NULL,
    address         VARCHAR(256) NOT NULL,
    tags            JSONB DEFAULT '[]',
    system_metrics  JSONB,
    active_tasks    INT DEFAULT 0,
    last_heartbeat  TIMESTAMPTZ NOT NULL,
    status          VARCHAR(16) DEFAULT 'ACTIVE',

    UNIQUE(app_id, worker_id)
);

-- 告警规则
CREATE TABLE alert_rules (
    id              BIGINT PRIMARY KEY,
    app_id          BIGINT NOT NULL, -- soft link: apps.id
    name            VARCHAR(256) NOT NULL,
    rule_type       VARCHAR(32) NOT NULL,
    condition       JSONB NOT NULL,
    channels        JSONB NOT NULL,
    enabled         BOOLEAN DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 审计日志
CREATE TABLE audit_logs (
    id              VARCHAR(64) PRIMARY KEY,
    actor           VARCHAR(128) NOT NULL,
    action          VARCHAR(64) NOT NULL,
    resource_type   VARCHAR(32) NOT NULL,
    resource_id     VARCHAR(128) NOT NULL,
    detail          JSONB,
    before          JSONB,
    after           JSONB,
    trace_id        VARCHAR(128),
    result          VARCHAR(32) NOT NULL DEFAULT 'success',
    failure_reason  TEXT,
    ip_address      VARCHAR(64),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 用户与权限
CREATE TABLE users (
    id              BIGINT PRIMARY KEY,
    username        VARCHAR(128) NOT NULL UNIQUE,
    email           VARCHAR(256),
    password   VARCHAR(256),
    oidc_subject    VARCHAR(256),
    role            VARCHAR(32) NOT NULL DEFAULT 'VIEWER',
    namespace_roles JSONB DEFAULT '{}',
    enabled         BOOLEAN DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 延迟任务队列
CREATE TABLE delayed_tasks (
    id              BIGINT PRIMARY KEY,
    job_id          BIGINT NOT NULL, -- soft link: jobs.id
    trigger_at      TIMESTAMPTZ NOT NULL,
    params          JSONB,
    outer_key       VARCHAR(256),
    status          VARCHAR(16) DEFAULT 'PENDING',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_delayed_tasks_trigger ON delayed_tasks(trigger_at) WHERE status = 'PENDING';
CREATE INDEX idx_instances_status ON instances(app_id, status) WHERE status IN ('WAITING', 'DISPATCHED', 'RUNNING');
CREATE INDEX idx_instances_job ON instances(job_id, created_at DESC);
CREATE INDEX idx_workers_heartbeat ON workers(app_id, last_heartbeat);
```

### 7.5 存储策略

| 数据 | 单机模式 | 集群模式 | 保留策略 |
|------|----------|----------|----------|
| 任务定义/配置 | SQLite | MySQL / PostgreSQL / CockroachDB | 永久 |
| 实例记录 | SQLite | MySQL / PostgreSQL / CockroachDB | 可配置 TTL（默认 30 天） |
| 实例日志 | SQLite + 文件 | DB + 对象存储 | 可配置 TTL（默认 7 天） |
| 工作流上下文 | SQLite | MySQL / PostgreSQL / CockroachDB | 随工作流实例清理 |
| Worker 心跳 | 仅内存 | DB + 内存缓存 | 实时，不持久化 |
| 延迟任务 | SQLite | MySQL / PostgreSQL / CockroachDB | 触发后归档 |
| 审计日志 | SQLite | MySQL / PostgreSQL / CockroachDB | 可配置 TTL（默认 365 天） |

---

## 8. 部署架构

tikeo 的部署目标是**容器优先、网络边界无关**。Server 和 Worker 必须可以部署在不同容器、不同 Docker 网络、不同 K8s namespace、不同 K8s 集群、不同 VPC/机房甚至不同云厂商中。平台不得要求 Worker 暴露入站端口；所有注册、心跳和反向调度指令都必须通过 Worker 主动建立的 tunnel 完成。

部署硬性约束：

1. **K8s 必须一等支持**：提供 Helm Chart、Kustomize 示例、Gateway API/Ingress 示例、ServiceMonitor、NetworkPolicy、PodDisruptionBudget 和多副本 StatefulSet/Deployment 模板。
2. **Docker 必须一等支持**：提供 server 镜像、worker 镜像、docker compose 示例、本地开发网络示例和 scratch/distroless 生产镜像。
3. **Server/Worker 可独立部署**：Server 可在平台集群，Worker 可在业务集群；Worker 可作为 sidecar、独立 Deployment、DaemonSet、Job Runner 或嵌入 SDK 运行。
4. **跨网络反向调用**：Server 不需要也不允许直接拨 Worker 地址；反向指令必须复用 Worker→Server 的长连接。
5. **单入口最小暴露**：默认只暴露 tikeo server 的 443/9090 tunnel/API 入口；业务命名空间默认不创建 Worker 入站 Service。

### 8.1 单机模式 (Standalone)

**零配置，开箱即用**——这是 tikeo 与 PowerJob 最大的用户体验差异。

```bash
# 下载单文件 (约 15MB，含前端)
curl -LO https://github.com/tikeo/tikeo/releases/latest/download/tikeo-linux-amd64
chmod +x tikeo-linux-amd64

# 启动 (自动创建 SQLite 数据库)
./tikeo-linux-amd64 serve

# 浏览器访问
open http://localhost:9090
```

**PowerJob 单机部署对比**：

```
PowerJob:
  1. 安装 JDK 8+
  2. 安装 MySQL
  3. 初始化数据库 schema
  4. 下载 powerjob-server.jar (~80MB)
  5. 配置 application.properties
  6. java -jar powerjob-server.jar
  7. 等待 Spring Boot 启动 (~15-30s)
  8. 访问 http://localhost:7700

tikeo:
  1. 下载 tikeo (~15MB)
  2. ./tikeo serve
  3. 浏览器自动打开 http://localhost:9090
  (启动时间 < 1s)
```

### 8.2 Docker / Compose 部署

Server 与 Worker 可以在同一个 Docker 网络内，也可以位于不同 Docker host。Worker 只需要配置 `TIKEO_SERVER` 指向可出站访问的 server tunnel endpoint。

```bash
# 单机 (约 20MB 镜像，基于 scratch)
docker run -d \
  --name tikeo \
  -p 9090:9090 \
  -v tikeo-data:/var/lib/tikeo \
  ghcr.io/tikeo/tikeo:latest

# 使用 MySQL
docker run -d \
  --name tikeo \
  -p 9090:9090 \
  -e TIKEO_DB_URL="mysql://user:pass@mysql:3306/tikeo" \
  ghcr.io/tikeo/tikeo:latest

# 使用 PostgreSQL
docker run -d \
  --name tikeo \
  -p 9090:9090 \
  -e TIKEO_DB_URL="postgres://user:pass@pg:5432/tikeo" \
  ghcr.io/tikeo/tikeo:latest

# Worker 独立容器：不暴露端口，只主动连 server
docker run -d \
  --name tikeo-worker \
  -e TIKEO_SERVER="https://tikeo.example.com" \
  -e TIKEO_APP_NAME="billing-worker" \
  -e TIKEO_WORKER_POOL="prod-cn" \
  ghcr.io/tikeo/tikeo-worker:latest
```

`docker compose` 推荐提供 `tikeo-server`、`tikeo-worker`、`postgres`、`prometheus` 四类服务模板；Worker 服务不声明 `ports`，只声明出站网络。

脚本能力 Worker Pool 必须独立部署并显式授权容器/子进程运行边界：

- Server 镜像/Pod 不挂载 Docker socket、不安装脚本运行时、不执行用户脚本。
- Shell/Python/JavaScript/TypeScript/PowerShell/Rhai Worker 广告统一 `script` 能力；具体语言 runner 与 backend 选择由 Worker 本地 registry/policy 决定。旧版 `script:shell`、`script:python`、`script:javascript`、`script:typescript`、`script:powershell`、`script:rhai` 仍被 Server 兼容为普通脚本 Worker；直接 WASM 模块使用 `script:wasm`；受控共享池才允许 `script:*` 或 `*`。
- 使用 `ContainerScriptRunner` 的 Worker 需要 Docker-compatible runtime 权限，应放在隔离 namespace/node pool，配合只读 rootfs、seccomp/AppArmor、NetworkPolicy、资源限额和独立 ServiceAccount。
- Worker 仅从 released immutable `script_versions` 快照执行，启动前校验 SHA-256，默认 `--network=none`、无宿主路径挂载、仅注入 tikeo 元数据 env 与策略白名单 env。


### 8.4 Server 集群 / Raft 实施边界

当前 Phase 2 已完成 `ClusterCoordinator` 抽象和 standalone coordinator：单节点模式返回 `role=standalone`，不会伪装为 Raft leader。真正 Raft 模式必须在同一抽象后实现，并满足：

- **Leader ownership gate**：只有 `Leader` 或显式 `Standalone` 能运行 CRON/fixed-rate tick loop、workflow materialize loop 和 dispatcher ownership-sensitive loop；当前代码已通过 `ClusterStatus.can_schedule` gate 保护 tick/dispatcher loop。
- **Follower fencing**：Follower 可以服务只读管理 API 和 Worker Tunnel 连接，但不能产生新调度 tick，不能越权抢占不属于自己的集群级 ownership。
- **DB conditional update remains required**：dispatch_queue lease/claim 的 DB 条件更新仍保留，用作 Raft 外的最后一道幂等/fencing 保护；`dispatch_queue.fencing_token` 已预留并随 claim 返回，后续可接入 Raft leader token。
- **Raft scope**：membership、term/index、leader lease、配置变更、调度 shard ownership；业务数据仍存储在 SeaORM 支持的数据库中，且继续禁止数据库外键。
- **Safe config shape first**：`cluster.mode/node_id/peers` 已可配置；`mode=raft` 只从 raft-rs runtime 观察角色，不能由配置态推导 leader，避免假 leader。
- **Raft metadata foundation**：`raft_metadata` 与 `raft_members` 已通过 SeaORM migration 建表，启动时可持久化本节点 term/index 初始元数据和静态 peers；表结构不包含外键，只通过 `node_id` 等字段软关联。
- **Raft durable records**：`raft_log_entries`、`raft_snapshots`、`raft_applied_commands` 已加入 SeaORM migration/entity/repository，用于 raft-rs `Ready` 流水线持久化 log entries、snapshot metadata/payload pointer 与状态机 apply 幂等记录；仍不创建数据库外键，仅按 `cluster_id/node_id/log_index/snapshot_index/command_id` 软关联。
- **Raft transport inbox**：`/api/v1/raft/append-entries` HTTP transport 形状适配 Docker bridge / K8s Service / LB/WAF 代理头；请求 DTO 已对齐 raft-rs `eraftpb::Message` 的 from/to/term/message_type/index/log_term/commit/entries/context/reject 字段，并在 route 层完成 message/entry type 校验、非负 index/term 校验和 base64 payload decode。当前在 raft runtime 存在时只负责投递到本地 runtime inbox，返回 `accepted=true` 仅表示队列接收成功，不表示 leader 授权或调度所有权；standalone/未启动 runtime 返回 `accepted=false` 和明确 reason。
- **Fencing token lifecycle**：`ClusterStatus` 与 `raft_metadata.leader_fencing_token` 已接入 leader fencing token 字段；token 只能由 raft-rs runtime 在观察到真实 `Leader` 且 term > 0 后生成并先写入 `raft_metadata`，非 leader 会清空 token。`can_schedule=true` 的前置条件是 token 已持久化且调度/分发 gate 能读取该 token；配置态/standalone Raft transport 不能生成 token。
- **Runtime implementation choice**：2026-05-21 改为集成 TiKV `raft-rs`（crate `raft` 0.7.0，Apache-2.0）。当前已在 `tikeo-server::cluster::raft_rs` 内完成 `RawNode` bootstrap/config/storage 边界校验：把现有字符串 `node_id` 通过 SHA-256 稳定映射为 raft-rs 需要的非 0 `u64` id，使用配置 peers 生成初始 voters，并构造 `MemStorage + RawNode` 证明依赖/API 可编译可运行。
- **No fake leadership**：raft-rs bootstrap/runtime ticker 仅证明 runtime 边界可创建与可驱动，会在无已知 leader 时由 runtime 自主触发 campaign，但不把配置态 Raft 解释为可调度 leader；`mode=raft` 带 storage 时可暴露 raft-rs 观察到的 follower/leader 角色，但只有真实 leader term + 已持久化 fencing token 才能打开 `can_schedule`。
- **Raft runtime ticker/inbox/outbound/apply skeleton**：`mode=raft` 且带 storage 启动时会创建 `RaftRuntimeCoordinator`，启动 100ms ticker 驱动 raft-rs `RawNode::tick()`，同时接收已校验的 HTTP 入站 `eraftpb::Message` 并调用 `RawNode::step()`；`Ready` 按 HardState -> log entries -> snapshot -> outbound messages -> committed entries apply bookkeeping -> `advance_append/advance_apply_to` 顺序处理，并同步更新 raft-rs `MemStorage` 的 hard_state/log/snapshot/commit；runtime 启动时会从 `raft_metadata` 与 `raft_log_entries` 恢复 HardState 和已持久化 entries，并先清空旧 `leader_fencing_token`，直到本轮真实 leader 观察重新生成 token，避免重启复用陈旧调度权。Outbound skeleton 会把 raft-rs `Message` 转回 HTTP wire DTO 并按配置 peer endpoint 追加 `/api/v1/raft/append-entries` 发送，支持 Docker bridge / K8s Service / LB/WAF 路径；可选 `cluster.transport_token` 会通过 `x-tikeo-raft-token` 做内部节点认证；HTTP route smoke 已验证正确 token 可绕过人工 session 并只代表 runtime inbox 入队，错误 token 仍回落到人工鉴权并返回标准 envelope。Committed `EntryNormal` 会写入 `raft_applied_commands` 幂等记录并推进 `raft_metadata.applied_index`；`noop` command type 视为已应用，未知命令记录为 `deferred_unsupported`，非法 JSON / 非法载荷记录为 `rejected`。首个真实业务命令为 `raft_member_upsert`，只更新 `raft_members` 成员目录元数据（`node_id/endpoint/status`），不触发 raft-rs `ConfChange`，且通过 `(cluster_id, command_id)` 做幂等回放去重。
- **Dynamic membership apply**：成员变更拆成两层：1) 普通 `EntryNormal` 的 `raft_member_upsert` 只维护平台可观测成员目录，允许 `configured/joining/active/leaving/removed` 等软状态；2) 真正改变 raft voters/learners 必须由 `/api/v1/raft/members:propose` 创建变更意图，且该接口要求当前节点是 real Raft Leader、`can_schedule=true`、已持久化 `leader_fencing_token`、调用者具备 `cluster:manage` 权限，并校验节点 id、http/https endpoint、移除当前 leader/破坏 quorum 风险。proposal 写入无外键 `raft_membership_proposals` 后投递 runtime command，由 raft-rs `propose_conf_change` 发起；committed `EntryConfChange/EntryConfChangeV2` 会被显式解码，并且只有 `RawNode::apply_conf_change` 成功、`raft_metadata.conf_state` 持久化成功后，才把 `raft_members` 状态推进到 `active/removed` 并把 proposal 标记为 `applied`。没有 runtime node、非法 payload、缺失 proposal context、或 unsupported multi-change V2 会被 gate/reject，不能静默变更 membership。当前已加入 deterministic in-process 3 节点 raft-rs harness，覆盖自主 tick 触发 election、真实 `campaign` leader election、leader fencing 持久化、ConfChange proposal commit/apply 到 `raft_membership_proposals` / `raft_metadata.conf_state` / `raft_members`。
- **Raft transport token**：`cluster.transport_token` 为可选 server-to-server shared token，应通过 Docker/K8s Secret 或环境变量 `TIKEO__CLUSTER__TRANSPORT_TOKEN` 注入，禁止提交生产 token。HTTP route 仍支持管理端 Bearer/RBAC；携带匹配 `x-tikeo-raft-token` 时可绕过人工 session，用于集群内部 Raft 消息。
- **Cluster diagnostics**：`/api/v1/cluster/diagnostics` 暴露当前 coordinator 状态、调度 gate、持久化 term/index/peer、transport 占位状态和 runtime boundary；`ClusterStatus.detail` 会包含 raft-rs bootstrap 校验摘要，便于 operator 判断为什么 Raft 节点尚未参与调度。
- **Container-first networking**：Raft 节点间通信必须可穿透 Docker bridge / K8s Service / LB，不能依赖 host network。`scripts/raft-bridge-e2e.sh` 会构建 alpine runtime 镜像、创建 Docker bridge 网络、用 container DNS (`tikeo-0:9090` 等) 写入 peers，并通过内部 `x-tikeo-raft-token` smoke-check `/healthz`、`/api/v1/cluster`、`/api/v1/cluster/diagnostics` 与 `/api/v1/raft/append-entries`；脚本同时校验最多一个可调度 leader 且必须带 fencing token。

### 8.3 Kubernetes 集群部署架构

```mermaid
graph TB
    subgraph K8s["Kubernetes Cluster"]
        direction TB

        LB["Ingress / LoadBalancer<br/>:443 TLS"]

        subgraph NS["Namespace: tikeo"]
            direction TB

            subgraph StatefulSet["tikeo-server (StatefulSet, 3 replicas)"]
                S1["Pod: Server 1<br/>port 9090<br/>Leader"]
                S2["Pod: Server 2<br/>port 9090<br/>Follower"]
                S3["Pod: Server 3<br/>port 9090<br/>Follower"]
            end

            SVC["Service: tikeo<br/>ClusterIP :9090"]

            subgraph CM["ConfigMap / Secrets"]
                CFG["tikeo-config<br/>TOML 配置"]
                SEC["tikeo-secrets<br/>DB 密码 / TLS 证书"]
            end

            PDB["PodDisruptionBudget<br/>minAvailable: 2"]
        end

        subgraph DB_NS["Namespace: database"]
            PG["PostgreSQL<br/>StatefulSet<br/>Primary + Replica"]
        end

        subgraph App_NS["Namespace: business-apps"]
            direction LR
            subgraph Pod1["Pod: app-a"]
                C1["app container<br/>:8080"]
                SC1["tikeo-worker<br/>sidecar"]
            end
            subgraph Pod2["Pod: app-b"]
                C2["app container<br/>:8080"]
                SC2["tikeo-worker<br/>sidecar"]
            end
        end
    end

    subgraph RemoteK8s["Remote Kubernetes / Docker / VM"]
        RW["remote tikeo-worker<br/>no inbound port"]
    end

    CLIENT["Client / CI/CD"] --> LB
    LB --> SVC
    SVC --> S1
    SVC --> S2
    SVC --> S3

    S1 --- PG
    S2 --- PG
    S3 --- PG

    SC1 -->|gRPC h2 tunnel outbound| SVC
    SC2 -->|gRPC h2 tunnel outbound| SVC
    RW -->|HTTPS/gRPC h2 tunnel outbound<br/>through NAT/Gateway| LB

    S1 ---|Raft| S2
    S2 ---|Raft| S3
    S3 ---|Raft| S1
```

### 8.4 Worker Sidecar 模式

tikeo Worker 支持以 Sidecar 模式运行在 K8s Pod 中，无需修改业务应用代码：

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-business-app
spec:
  template:
    spec:
      containers:
      - name: app
        image: my-business-app:latest
      - name: tikeo-worker
        image: ghcr.io/tikeo/tikeo-worker:latest
        env:
        - name: TIKEO_SERVER
          value: "https://tikeo.tikeo.svc.cluster.local:9090"
        - name: TIKEO_APP_NAME
          value: "my-business-app"
        - name: TIKEO_PROCESSORS
          value: "http://localhost:8080/tasks/*"
```

### 8.5 Worker 部署形态

| 形态 | 适用场景 | 网络要求 | 备注 |
|------|----------|----------|------|
| SDK 嵌入业务进程 | 业务代码直接实现处理器 | 业务进程出站访问 tikeo | 延迟最低，适合核心业务任务；Java/Rust SDK 已按服务端下发 worker_id 模型接入 Worker Tunnel |
| Sidecar | 不希望业务进程直接管理调度连接 | Pod 内 localhost 调业务容器；sidecar 出站访问 tikeo | 默认 K8s 推荐模式 |
| 独立 Worker Deployment | HTTP/gRPC/SQL/Script 等通用任务 | Worker 出站访问 tikeo 和目标系统 | 适合共享 worker pool |
| DaemonSet | 节点级任务、文件清理、宿主观测 | 每节点 Worker 出站访问 tikeo | 需更严格权限策略 |
| 跨集群 Worker Gateway | 远端集群集中接入 | gateway 出站访问中心 tikeo | 可减少远端集群出口连接数 |
| 容器化脚本 Runner | 动态脚本/依赖复杂任务 | runner 出站访问 tikeo | 配合 seccomp/AppArmor/NetworkPolicy |

### 8.6 完整生产部署拓扑

```mermaid
graph TB
    subgraph External["外部访问"]
        USER["用户 / 运维"]
        CI["CI/CD Pipeline"]
        MON["Grafana / Prometheus"]
    end

    subgraph DMZ["DMZ / Ingress"]
        ING["Nginx Ingress<br/>TLS 终结"]
    end

    subgraph Prod["生产环境"]
        direction TB

        subgraph tikeo["tikeo 集群"]
            S1["Server 1<br/>Leader"]
            S2["Server 2"]
            S3["Server 3"]
        end

        subgraph DB_Layer["数据层"]
            PG_M["PostgreSQL<br/>Primary"]
            PG_R1["PostgreSQL<br/>Read Replica"]
            REDIS["Redis<br/>(可选缓存)"]
        end

        subgraph Workers["Worker 集群"]
            direction LR
            WA["Worker A<br/>Rust<br/>Pod x 5"]
            WB["Worker B<br/>Go<br/>Pod x 3"]
            WC["Worker C<br/>Python<br/>Pod x 2"]
        end
    end

    subgraph Storage["持久化"]
        OBJ["对象存储 S3/MinIO<br/>日志归档"]
        NFS["共享存储<br/>WASM 容器包"]
    end

    USER -->|HTTPS| ING
    CI -->|gRPC| ING
    MON -->|Prometheus| tikeo

    ING --> tikeo

    S1 --- PG_M
    S2 --- PG_M
    S3 --- PG_M
    PG_M --> PG_R1

    WA -->|gRPC| tikeo
    WB -->|gRPC| tikeo
    WC -->|gRPC| tikeo

    tikeo --> OBJ
    tikeo --> NFS
```

### 8.7 K8s / Docker 公共服务化部署要点

本设计明确把“公共调度服务”作为部署目标，而不是只服务单个业务系统。与 xxl-job / PowerJob 相比，tikeo 在 K8s、Docker 和跨集群容器部署中的关键差异如下：

| 场景 | xxl-job / PowerJob 的问题 | tikeo 方案 |
|------|---------------------------|----------------|
| 同集群多 namespace | 每个 Executor/Worker 都要暴露可回连地址，NetworkPolicy 和 Service 管理复杂 | Worker 只出站连接 tikeo Service，不要求业务 namespace 暴露调度端口 |
| 多集群/多 VPC | 中心调度服务要能路由到远端 Pod/Service | 远端 Worker 主动连接中心或区域 gateway；Server 通过既有 tunnel 反向下发任务，天然跨 NAT/防火墙/网关 |
| 业务 Pod 重启/扩缩容 | Executor/Worker 地址变化导致注册表与实际可达性不一致 | 连接断开即下线，重新连接即注册，租约过期自动重调度 |
| Service Mesh | 多协议、多端口、多方向流量策略复杂 | 单 gRPC h2 出站连接，mTLS/限流/审计集中治理 |
| 本地 Docker/Compose 调试 | external address/port 很容易配置错误 | 只需配置 `TIKEO_SERVER`，Worker 容器不声明入站 ports，不需要暴露给 Server |

因此 Helm Chart 和 Compose 模板默认不为业务 Worker 创建入站 Service/ports；只有 tikeo server 暴露管理面和 Worker Tunnel 入口。

---

## 9. 数据管道与事件流

### 9.1 任务事件管道

```mermaid
flowchart LR
    subgraph Sources["事件源"]
        E1["任务触发"]
        E2["任务完成"]
        E3["任务失败"]
        E4["Worker 上线/下线"]
        E5["工作流状态变更"]
    end

    subgraph Bus["Event Bus<br/>(tokio::broadcast)"]
        CH["Channel"]
    end

    Sources --> CH

    subgraph Consumers["消费者"]
        C1["状态更新 → DB"]
        C2["日志写入 → 存储"]
        C3["指标聚合 → Prometheus"]
        C4["告警评估 → Alert Engine"]
        C5["WebSocket 推送 → UI"]
        C6["审计记录 → audit_logs"]
        C7["OTLP Span → Jaeger/Tempo"]
    end

    CH --> C1
    CH --> C2
    CH --> C3
    CH --> C4
    CH --> C5
    CH --> C6
    CH --> C7
```

### 9.2 日志收集管道

```mermaid
flowchart LR
    subgraph Worker["Worker 端"]
        EXEC["任务执行<br/>输出日志"]
        BUF["内存 RingBuffer<br/>容量可配"]
    end

    EXEC --> BUF

    BUF -->|gRPC Client Stream<br/>背压控制| GW["Server Gateway"]

    GW --> DISP["日志分发器"]

    DISP -->|实时流| WS["WebSocket<br/>→ 浏览器 UI"]
    DISP -->|批量写入| DB["Database<br/>实例日志表"]
    DISP -->|异步归档| S3["对象存储<br/>历史日志"]

    DISP -->|结构化| OTEL["OpenTelemetry<br/>→ Loki / Elastic"]
```

### 9.3 告警评估管道

```mermaid
flowchart TD
    EVENT["任务事件<br/>FAILED / TIMEOUT"] --> EVAL["规则引擎<br/>评估告警条件"]

    EVAL -->|匹配规则| ENRICH["上下文丰富<br/>附加 Job/Worker 信息"]
    ENRICH --> DEDUP["去重 & 静默<br/>防止告警风暴"]
    DEDUP --> ROUTE{"告警路由"}

    ROUTE -->|P0 关键| PD["PagerDuty<br/>电话/短信"]
    ROUTE -->|P1 重要| SLACK["Slack / 飞书 / 钉钉"]
    ROUTE -->|P2 一般| EMAIL["邮件通知"]
    ROUTE -->|P3 信息| WEBHOOK["Webhook<br/>自定义处理"]

    ROUTE --> AUDIT["审计日志记录"]
```

---

## 10. 安全模型

### 10.1 安全架构分层

```mermaid
graph TD
    subgraph L5["Layer 5: 审计"]
        AUD["全操作审计日志<br/>不可篡改 / SIEM 导出"]
    end

    subgraph L4["Layer 4: 执行隔离"]
        WASM["WASM 沙箱<br/>用户代码"]
        SUB["脚本沙箱<br/>Shell / Python / Node / PowerShell"]
        RLIM["资源限制<br/>CPU / 内存 / 超时 / 输出大小"]
    end

    subgraph L3["Layer 3: 授权 RBAC"]
        ADMIN["Admin: 全部权限"]
        OP["Operator: 创建/触发/取消"]
        VIEW["Viewer: 只读"]
        NS["Namespace 级别隔离"]
    end

    subgraph L2["Layer 2: 认证"]
        OIDC["OIDC / SSO"]
        TOKEN["API Token"]
        HMAC["App Key + HMAC<br/>Worker 认证"]
    end

    subgraph L1["Layer 1: 传输"]
        MTLS_W["mTLS<br/>Server ↔ Worker"]
        MTLS_S["mTLS<br/>Server ↔ Server"]
        TLS_C["TLS + Token<br/>Client ↔ Server"]
    end

    L1 --> L2 --> L3 --> L4 --> L5
```

### 10.2 对比 xxl-job / PowerJob 的安全边界

| 安全域 | xxl-job 问题 | PowerJob 问题 | tikeo 解决方案 |
|--------|--------------|---------------|--------------------|
| 远程代码执行 | GLUE_GROOVY、Shell/Python/JavaScript/TypeScript/PowerShell 等在宿主执行 | GroovyEvaluator 决策节点、动态脚本、外部 JAR 容器扩大攻击面 | 多语言脚本沙箱 + WASM/容器隔离；Server 不执行用户代码；脚本签名、审批、能力声明与最小权限 |
| SQL 注入/危险 SQL | 核心平台较少内置 SQL 任务，但缺统一 SQL 治理 | `detailPlus` customQuery 黑名单过滤后拼接；SQL Processor 默认依赖用户注册 validator | 参数化 SQL 模板、数据源白名单、dry-run、审批、审计 |
| SSRF/内网探测 | HTTP 类任务缺平台级 egress policy | HTTP Processor 允许任务参数指定 URL，默认缺内网地址治理 | URL 白名单/黑名单、DNS pinning、禁止 metadata/link-local/内网网段 |
| 传输认证 | 默认 `default_token`，可为空 | Worker/Server 多协议通信缺统一 mTLS 默认模型 | TLS/mTLS、Worker cert rotation、bootstrap token 最小权限 |
| 权限模型 | 用户/执行器维度较粗 | V5.x 权限增强但 OpenAPI/控制台/Worker 链路仍不统一 | gRPC/REST 方法级鉴权，namespace/app/worker pool scope |
| 凭证治理 | 配置文件明文较常见 | 配置与任务参数容易携带明文凭证 | Secret reference、Vault/KMS/K8s Secret、日志脱敏 |
| 审计 | 操作审计不足 | 审计不完整 | 全操作审计 + SIEM/OTLP 导出 |
| 网络暴露 | Executor 必须暴露入站端口 | Worker 必须暴露入站端口，多端口多协议 | Worker 仅出站连接 tikeo，减少攻击面 |

---

## 11. 性能分析

### 11.1 预期性能指标

| 指标 | PowerJob (Java/Akka) | tikeo (Rust/gRPC) | 预期提升 |
|------|---------------------|---------------------|----------|
| Server 启动时间 | 15-30s (Spring Boot) | < 1s (原生二进制) | **15-30x** |
| Server 内存占用 | 512MB-2GB (JVM) | 32-128MB | **5-15x** |
| Worker SDK 体积 | ~50MB (JAR + 依赖) | ~5MB (Rust) / ~10MB (Go) | **5-10x** |
| Docker 镜像大小 | ~500MB (Java) | ~20MB (scratch) | **25x** |
| 调度延迟 (P50) | ~5ms | ~1ms | **5x** |
| 调度延迟 (P99) | ~25-235ms | ~5-30ms | **5-8x** |
| 单节点吞吐量 | ~5K tasks/s | ~50K+ tasks/s | **10x** |
| Worker 心跳间隔 | 15s (可调) | 10s (默认，可调) | 更快故障检测 |
| 日志传输 | 批量上报，溢出丢日志 | gRPC 流，背压控制，零丢失 | 质变 |
| CPU 效率 | GC 暂停，JIT 预热 | 零 GC，稳定延迟 | 质变 |

### 11.2 性能关键设计

1. **无锁调度**：Tokio 的无锁 channel 替代锁
2. **零拷贝序列化**：protobuf 解析使用 bytes::Bytes 引用计数
3. **io_uring**：Linux 下使用 tokio-uring 进行异步 I/O
4. **连接池化**：gRPC 连接复用，避免频繁建连
5. **内存分配器**：jemalloc 或 mimalloc，减少内存碎片

---

## 12. 可观测性

### 12.1 三大支柱

```mermaid
graph TB
    subgraph Sources["数据采集"]
        SCH["Tikeo<br/>调度事件"]
        WORK["Workers<br/>执行指标"]
        GW["Gateway<br/>API 延迟"]
        DB["Database<br/>查询耗时"]
    end

    subgraph Pillars["三大支柱"]
        MET["Metrics<br/>Prometheus format"]
        LOG["Logging<br/>tracing crate"]
        TRACE["Tracing<br/>OpenTelemetry OTLP"]
    end

    Sources --> Pillars

    subgraph Backends["可视化后端"]
        GRAFANA["Grafana<br/>Dashboard"]
        LOKI["Loki / Elastic<br/>日志检索"]
        JAEGER["Jaeger / Tempo<br/>分布式追踪"]
    end

    MET --> GRAFANA
    LOG --> LOKI
    TRACE --> JAEGER
```

### 12.2 内置指标 (Prometheus)

```
# 调度/实例 SLO 指标
tikeo_tikeo_tasks_dispatched_total{app, job_type}
tikeo_tikeo_tasks_succeeded_total{app, job_type}
tikeo_tikeo_tasks_failed_total{app, job_type}
tikeo_tikeo_dispatch_duration_seconds{app}        # histogram
tikeo_job_instances_current{status}
tikeo_job_instance_success_ratio

# 队列指标
tikeo_tikeo_queue_length{app}
tikeo_tikeo_queue_capacity{app}
tikeo_dispatch_queue_pending_age_seconds{stat="oldest|average"}    # histogram
tikeo_dispatch_queue_items_total{status="pending|running"}

# Worker 指标
tikeo_worker_active_count{app}
tikeo_workers_online_current
tikeo_worker_heartbeat_latency_seconds{app, worker}
tikeo_worker_task_duration_seconds{app, processor}    # histogram

# 告警/治理指标
tikeo_alert_events_current{status}
tikeo_script_governance_failures_current{failure_class}

# 系统指标
tikeo_server_uptime_seconds
tikeo_server_connections_active
tikeo_db_query_duration_seconds{operation}            # histogram
tikeo_grpc_request_duration_seconds{method}           # histogram
```

---

## 13. 技术栈

### 13.1 核心依赖

依赖版本策略：新项目默认使用当前最新稳定版依赖；若因兼容性、许可证、安全策略或生态稳定性不能使用最新版，必须在决策记录中说明原因、锁定版本、风险和升级条件。

| 组件 | 技术选型 | 版本 | 用途 |
|------|----------|------|------|
| 语言 | Rust | 2024 Edition | 整体实现 |
| 异步运行时 | Tokio | 1.x | 异步 I/O、任务调度、时间 |
| gRPC 框架 | Tonic | 0.12+ | gRPC server/client |
| HTTP 框架 | Axum | 0.8+ | REST API、Web 控制台 |
| protobuf | Prost | 0.13+ | Protocol Buffers 编解码 |
| **ORM** | **SeaORM** | **1.1+** | **多数据库异步 ORM，SQLite/MySQL/Pg/CockroachDB** |
| 共识算法 | TiKV raft-rs (`raft`) | 0.7.x | Server 集群 Raft 共识；当前已接入 bootstrap、ticker、Ready 持久化顺序、inbound inbox 与 outbound HTTP skeleton，apply/fencing/membership 继续推进 |
| WASM 运行时 | Wasmtime | 25+ | 用户代码沙箱 |
| CLI 框架 | Clap | 4.x | 命令行解析 |
| 配置 | config-rs | 0.14+ | TOML/YAML/ENV 配置 |
| 序列化 | Serde | 1.x | JSON/TOML/YAML 序列化 |
| 日志 | tracing | 0.1.x | 结构化日志 |
| 指标 | metrics + metrics-exporter-prometheus | 0.24+ | Prometheus 指标导出 |
| 追踪 | opentelemetry-rust | 0.27+ | OTLP 追踪导出 |
| 前端 | React + TypeScript + Vite + Ant Design | 当前最新稳定版 | `./web` Web 管理控制台 |
| 前端包管理 | Bun | 当前最新稳定版 | Web 依赖管理、脚本运行、测试、构建 |
| 嵌入静态资源 | include_dir | 0.7+ | 编译时嵌入 `./web` 构建产物 |
| OpenAPI | utoipa / aide / schemars | 当前最新稳定版 | OpenAPI 3.1、JSON Schema；不提供浏览器文档 UI |
| 实时推送 | SSE / WebSocket | — | 实时日志、状态事件、Dashboard 刷新 |

### 13.2 开发工具链

| 工具 | 用途 |
|------|------|
| cargo | 构建管理 |
| cargo-nextest | 测试运行器 |
| sea-orm-cli | Entity 生成 + 数据库迁移 |
| tonic-build | protobuf 代码生成 |
| criterion | 性能基准测试 |
| cargo-deny | 许可证和安全审计 |
| clippy | 代码质量检查 |

---

## 14. 项目结构

```
tikeo/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── clippy.toml
├── deny.toml
│
├── proto/
│   ├── tikeo/
│   │   ├── worker/v1/
│   │   │   ├── worker.proto
│   │   │   ├── task.proto
│   │   │   └── processor.proto
│   │   ├── api/v1/
│   │   │   ├── job.proto
│   │   │   ├── workflow.proto
│   │   │   └── admin.proto
│   │   └── raft/v1/
│   │       └── raft.proto
│   └── buf.yaml
│
├── crates/                            # 所有 Rust 模块 crate 统一放在此目录，workspace 解耦开发
│   ├── tikeo-server/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── config.rs
│   │   │   ├── server.rs
│   │   │   ├── tikeo/
│   │   │   ├── workflow/
│   │   │   ├── cluster/
│   │   │   ├── storage/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── entities/          # SeaORM Entity
│   │   │   │   ├── repositories/      # Repository 抽象
│   │   │   │   └── migration/         # SeaORM Migration
│   │   │   ├── gateway/
│   │   │   ├── auth/
│   │   │   ├── alert/
│   │   │   ├── observability/
│   │   └── migration/                 # SeaORM 迁移文件
│   │       ├── m20250101_000001_namespace.rs
│   │       ├── m20250101_000002_app.rs
│   │       ├── m20250101_000003_job.rs
│   │       └── ...
│   │
│   ├── tikeo-sdk/
│   ├── tikeo-client/
│   ├── tikeo-proto/
│   ├── tikeo-common/
│   └── tikeo-wasm/              # Worker 侧 Wasmtime 执行器；不耦合 server HTTP/storage
│
├── sdks/                             # 多语言 SDK
│   ├── rust/tikeo/    # Rust Worker SDK crate
│   ├── java/tikeo/           # 原生 Java SDK
│   ├── java/tikeo-spring/         # Spring 集成
│   ├── java/tikeo-spring-boot-starter/    # Spring Boot 集成
│   ├── go/tikeo-go-sdk/           # 规划
│   ├── python/tikeo-python-sdk/   # 规划
│   └── nodejs/tikeo-nodejs-sdk/   # 规划
│
├── examples/                         # SDK demo 项目，按 sdks/ 语言结构对齐
│   ├── rust/worker-demo/
│   ├── java/spring-boot2-worker-demo/
│   ├── java/spring-boot3-worker-demo/
│   ├── java/spring-boot4-worker-demo/
│   ├── go/worker-demo/
│   ├── python/worker-demo/
│   └── nodejs/worker-demo/
│
├── web/                              # React + Ant Design + Bun 管理控制台
├── deploy/
│   ├── docker/
│   ├── k8s/
│   └── helm/                         # Helm Chart
│
├── tests/
├── docs/
└── config/
```

---

## 15. 开发路线图

### Phase 1: MVP (月 1-3)

**目标**：核心调度能力，可替换 PowerJob 的基本使用场景。

- [x] 项目脚手架 (workspace, CI, root binary entrypoint；2026-06-05 主 CI 已覆盖 Server/Web/Java/Go/Rust SDK 与 demo、跨语言 Worker smoke、Docker build，并通过 `Workflow policy` 禁止 Node.js 20 或更旧 GitHub JavaScript action runtime 回归)
- [x] gRPC 协议定义与代码生成（Worker Tunnel proto + server streaming skeleton）
- [x] SeaORM 存储层 + SQLite + MySQL 迁移（SQLite dev DB 已验证，MySQL migration 通过 SeaORM feature 启用；2026-06-05：SQLite legacy schema 兼容升级迁入显式 SeaORM migration `sqlite_compat` 并由 `seaql_migrations` 记录）
- [x] 基础调度器 (CRON + Fixed Rate + API 触发)
  - [x] API 手动触发实例链路（创建 pending job_instance + 实例列表/详情查询）
  - [x] CRON tick loop（cron 0.16 expression + in-memory trigger cursor）
  - [x] Fixed Rate tick loop（humantime duration expression + in-memory trigger cursor）
- [x] 单机执行 + 广播执行
  - [x] 最小单机执行链路（pending instance -> first available worker -> running/succeeded/failed）
  - [x] 广播执行
  - [x] 基于 namespace 和 app 的 Worker 能力路由
- [x] Rust SDK (Worker 注册 + 心跳 + 任务执行)
  - [x] Rust Worker SDK 最小主动连接/注册/心跳客户端
  - [x] 基础 TaskProcessor trait / TaskContext / TaskOutcome
  - [x] Worker 真实任务接收与执行回传（Worker Tunnel dispatch + SDK TaskProcessor + result status update）
- [x] 基础 HTTP REST API skeleton（统一 `{code,message,data}` 响应、system/cluster/jobs 占位）
- [x] OpenAPI 3.1 JSON 文档（`/api-docs/openapi.json`；不提供文档 UI）
- [x] 基础 Web UI (登录、Dashboard、Job 列表、创建、手动触发、实例详情、日志查看)
  - [x] Web 工程基础：React + TypeScript + Vite + Ant Design + Bun
  - [x] Dashboard / Job 创建列表 / 手动触发 / 实例列表骨架
  - [x] 登录与权限感知操作
  - [x] 用户管理与基础 RBAC（Users API、Admin 管理界面、角色校验）
  - [x] RBAC permission/resource/action 模型（roles / permissions / role_permissions 软关联，无外键）
  - [x] 后端大文件模块化拆分（HTTP routes 与 storage repository 按领域拆分）
  - [x] 可插拔 SessionStore 抽象（当前 DB + moka，预留 Redis 分布式实现）
  - [x] 实例日志查看（Worker TaskLog -> storage -> HTTP logs API -> Web Drawer）
- [x] Docker 镜像构建（server 多阶段镜像 + Web nginx 镜像 + Compose/K8s 基础部署；CI 中 server/web 镜像校验拆成 `Docker build validation / server` 与 `Docker build validation / web` 两个并行 job，使用 Buildx `type=gha` cache，只构建不推送）
- [x] CLI 基础命令（`serve --config`）

### Phase 2: 工作流与分布式 (月 4-6)

**目标**：覆盖 PowerJob 的全部调度模式。

- [x] DAG 工作流引擎基础（定义存储、DAG 校验、最小 run API；可视化编排后续增强）
- [x] Map / MapReduce 执行模式（workflow_shards + materialize + shard job_instance/dispatch_queue 软关联）
- [x] 子工作流嵌套（节点引用 child_workflow_id + 子实例软关联 + 子实例终态回写父节点）
- [x] PostgreSQL + CockroachDB 存储支持（SeaORM/sqlx-postgres feature + `postgres://` 配置模板；CockroachDB 复用 PostgreSQL wire protocol）
- [x] Server 集群（Raft 共识安全基础；TiKV raft-rs 已完成 bootstrap/ticker/inbound/outbound/apply/fencing/membership 与 Docker bridge smoke，生产级快照压缩/Chaos 后续增强）
  - [x] ClusterCoordinator 抽象与显式 standalone 状态（`/api/v1/cluster` 不再伪装 leader）
  - [x] tick/dispatcher ownership gate（非 `can_schedule` 节点跳过 CRON/fixed-rate tick 与 Worker dispatch loop）
  - [x] Raft 配置形状（mode/node_id/peers）与未启动 Raft 的 unknown/not-schedulable 状态
  - [x] Raft metadata/member 持久化基础（`raft_metadata` / `raft_members`，无外键，启动时写入配置 peers）
  - [x] Raft transport/fencing 形状（`/api/v1/raft/append-entries` 占位接口 + `leader_fencing_token` 字段，仍不授予 leader）
  - [x] Cluster diagnostics（`/api/v1/cluster/diagnostics` 展示 gate、term/index、peers、transport 占位和 runtime boundary）
  - [x] TiKV raft-rs (`raft` 0.7.0) 依赖接入与 `RawNode` bootstrap 校验（不启动 event loop，不授予 leader）
  - [x] raft-rs durable record 基础（`raft_log_entries` / `raft_snapshots`，无外键，Repository upsert/list 覆盖 Ready 后续持久化入口）
  - [x] raft-rs message transport DTO + 转换校验基础（`/api/v1/raft/append-entries` 请求对齐 from/to/term/message_type/index/log_term/commit/entries/context/reject，可转换为 `eraftpb::Message`）
  - [x] raft-rs runtime ticker + Ready 持久化顺序骨架（tick -> Ready HardState/log/snapshot 持久化 -> advance；不 campaign，不 outbound transport，不授予 scheduling）
  - [x] raft-rs inbound runtime inbox 接入（HTTP 校验后投递 runtime mpsc inbox；`accepted=true` 仅表示本地队列接收成功，不授予 scheduling）
  - [x] raft-rs outbound message transport skeleton（Ready messages -> HTTP wire DTO -> peer endpoint `/api/v1/raft/append-entries`；支持可选 `cluster.transport_token` / `x-tikeo-raft-token`，不授予 scheduling）
  - [x] raft-rs Ready apply bookkeeping + fencing token lifecycle（committed `EntryNormal` 推进 `raft_metadata.applied_index`；config-change entries 显式 gate；真实 Leader term 先持久化 token 后才允许 `can_schedule`）
  - [x] raft-rs 业务 apply 命令信封与幂等记录（`raft_applied_commands`，先支持 `noop`，未知命令 deferred，非法 payload rejected；无外键）
  - [x] raft-rs 首个真实业务命令 apply（`raft_member_upsert` 更新 `raft_members` 成员目录元数据；按 `command_id` 幂等回放去重；不触发 ConfChange）
  - [x] raft-rs 动态 membership/config change 流程设计（member catalog 与真正 raft voters/learners 变更拆层；未实现前继续 gate config-change entries）
  - [x] raft-rs dynamic membership proposal API 意图层（`/api/v1/raft/members:propose`，要求 `cluster:manage` + real Leader + persisted fencing token；写入 `raft_membership_proposals`，无外键）
  - [x] raft-rs committed ConfChange/ConfChangeV2 解码、`ConfState` 持久化与 membership 状态推进（runtime node 存在时 apply；无 runtime/非法 payload/unsupported V2 保持 gate/reject）
  - [x] raft-rs 多节点实测 campaign/leader election + membership proposal e2e 验证（in-process 3 节点 RawNode harness，真实 campaign，无 fake leadership）
  - [x] raft-rs runtime 重启恢复硬化（从 `raft_metadata` / `raft_log_entries` 恢复 MemStorage HardState/log entries，并清空 stale leader fencing token）
  - [x] raft-rs HTTP transport token/auth/envelope smoke（本地 route harness 覆盖 `x-tikeo-raft-token` 免人工 session、错误 token 拒绝、runtime inbox accepted=true 语义）
  - [x] raft-rs Docker bridge / K8s Service 多容器 E2E 脚本化验证（`scripts/raft-bridge-e2e.sh`，bridge 网络 + container DNS + token + health/cluster/diagnostics/append smoke；不使用 host network）
- [x] 任务队列基础（dispatch_queue 持久化模型、priority/run_after/status/lease_owner/lease_until 字段；workflow queued node 自动 materialize）
- [x] 持久化延迟队列基础（dispatch_queue.run_after）
- [x] 实时日志流 (gRPC Server Stream：`SubscribeTaskLogs` 支持历史回放 + Worker Tunnel live fan-out)
- [x] 工作流可视化编辑器（基础 DAG 预览 + 节点状态着色）
- [x] Web UI 工作流 JSON 定义入口、YAML 预览、dry-run、validate/run、SSE、shards 和恢复入口基础
- [x] Workflow executor 最小推进能力（`advance` 按节点状态与边条件推进后继 waiting 节点到 queued，并写入 dispatch_queue 与 instance_events）
- [x] Workflow 定义约束补齐（Map/MapReduce 节点要求 `map_items`，子工作流节点要求 `child_workflow_id`）
- [x] Workflow queued node 物化执行（`materialize-next` 将 queued 节点物化为 job_instance、workflow_shards 或 child workflow instance）
- [x] Workflow 节点恢复 API（`recover` 支持 retry/skip/fail/succeed 基础恢复语义）
- [x] Worker / dispatch queue 管理 API 与 Web Worker 集群页面
  - [x] Worker 集群页面运维布局重做（100：数据密集 dashboard；Worker table 支持 search/namespace/capability 筛选；Dispatch Queue 支持状态 drill-down；队列压力/健康状态卡片；拆分为 focused React components）
  - [x] Worker 集群页面按 namespace/app 与 cluster/region 分组（2026-06-04：主页面只展示 Worker 集群/node 列表和 master/follower；调度队列迁移到 `/workers/dispatch-queue` 二级页，避免 Worker 集群视图混杂队列明细）
- [x] Worker TaskResult 自动推进 Workflow（按 job_instance_id 软关联回写 workflow_node_instance / workflow_shard，并按边条件入队后继节点）
- [x] Workflow shard 完成回调与聚合推进（`POST /api/v1/workflow-shards/{id}/complete` 写入 output/status，全部成功后自动推进后继，失败时走失败边）
- [x] Workflow 操作审计日志（create/update/validate/dry-run/run/advance/materialize/recover 管理与执行动作写入 audit_logs）
- [x] Dispatch queue 最小租约与 claim API（lease_owner / lease_until + SQLite 兼容迁移；`POST /api/v1/dispatch-queue:claim` 支持按租约占用队列项）
- [x] Dispatch queue 原子 claim 与 dispatcher 接入（DB 条件更新抢占租约、过期 pending lease 回收、workflow queued node 和 single job dispatch 统一走 dispatch_queue）
- [x] Dispatch queue fencing token 形状（`fencing_token` 随 claim 写入/返回，dispatcher 使用 cluster-derived token，后续接 Raft leader token）
- [x] SSE 实时实例事件骨架（instance_events + /events/instances/:id/stream；WebSocket 后续）

### Phase 3: 企业级特性 (月 7-9)

**目标**：可安全地在生产环境大规模部署。

> 020 review remediation 结论：015-019 中已完成项若仅为骨架，路线图必须明确标注“骨架/基础”，不得把未接入真实执行链路、规则引擎或治理闭环的能力标为完全完成。

- [x] RBAC 权限系统（021 已完成最小 permission/resource/action；OIDC/多租户 scope 后续继续增强）
  - [x] 角色管理模块与权限矩阵后台（149：内置 owner 角色锁定且仅初始化账号专属、bootstrap_admin 结构化 owner bypass、自定义角色 CRUD、用户角色绑定、后端接口权限矩阵、菜单权限矩阵、UI 操作元素权限矩阵、角色/用户权限配置 UI 与自动化测试）
  - [x] API Token 生命周期基础（098：`POST/GET/DELETE /api/v1/auth/api-tokens`，token 只创建时返回明文，持久化仍为哈希；列表不暴露 token_hash，删除后 bearer 立即失效并审计）
  - [x] API Token 细粒度 scope 基础（099：创建时可传 `scopes=["resource:action"]`；scope 会收窄 effective permissions，`admin` role 不再绕过 scoped token 限制；列表返回 scopes，scope 校验不得超过当前 principal 权限）
  - [x] API Token 过期/轮换策略基础（102：`auth.api_tokens` 配置 default/min/max TTL；创建可传受策略约束的 `expires_in_seconds`；`POST /api/v1/auth/api-tokens/{id}/rotate` 保留 scopes、签发新 token 并立即撤销旧 token）
  - [x] API Token namespace/app/worker-pool scope binding 基础（104：创建 token 可传 `scope_bindings`；token metadata 与 `/auth/me` 返回绑定；jobs list/create/trigger 按 namespace/app 过滤或拒绝；workers list 按 namespace/app/worker_pool label 过滤）
- [x] OIDC/SSO 集成
  - [x] OIDC/SSO 配置与状态基础（085：`auth.oidc` 配置、`GET /api/v1/auth/status` 暴露本地/oidc 模式与脱敏 provider 元数据）
  - [x] OIDC 授权/回调骨架（092：`GET /api/v1/auth/oidc/authorize` 生成授权 URL 且不暴露 secret；`/callback` 校验 code/state 形状但明确拒绝未验证 token，不创建 session）
  - [x] OIDC token exchange 边界（116：callback 使用配置的 client credentials 调用 provider token endpoint，要求返回 `access_token`，但在 UserInfo/signature/claims 验证和用户映射前仍 fail-closed 不创建 session）
  - [x] OIDC UserInfo 获取边界（117 修正：callback token exchange 后读取 provider discovery 的 `userinfo_endpoint` 并获取外部 subject；在本地 user/role/tenant 映射前仍 fail-closed，不创建 tikeo opaque session）
  - [x] OIDC 外部身份映射与 opaque session 签发（122：`oidc_identities` 以 `(issuer, subject)` 软链接本地 username 和 namespace/app/worker_pool scope；callback 只用 provider access token 读取 UserInfo，映射命中后签发本地 `auth_sessions` opaque bearer token，继续严禁 JWT 作为本地登录态）
- [x] mTLS 传输加密
  - [x] TLS/mTLS 配置与诊断基础（086：`transport_security` 配置、`GET /api/v1/security/transport` 脱敏显示 HTTP/Worker Tunnel TLS/mTLS readiness）
  - [x] TLS listener 边界 fail-closed（094：状态返回 `listener_mode=plaintext|tls_pending_listener`；TLS/mTLS 开启时即使证书路径齐全也标记 not ready，直到真实监听器 TLS wiring 完成）
  - [x] 真实 HTTP 与 Worker Tunnel TLS/mTLS listener（123：HTTP 使用 rustls 实际 HTTPS listener，按新连接重载证书文件以支持轮换；Worker Tunnel 使用 tonic TLS/mTLS listener；诊断从 `tls_pending_listener` 更新为 `tls|mtls|tls_config_error` 并校验证书文件可读）
- [x] Web 前端路由与导航治理基础（React Router v7、路由守卫、URL 持久化、菜单与路由对齐）
  - [x] 路由 meta、懒加载、统一 403/401 与 URL 查询参数治理（`web/src/routes.tsx` 单一元信息源；页面 lazy chunks；API client 统一 401 清 token 跳登录、403 跳禁止页；审计/任务/脚本/工作流列表查询状态进入 URL）
- [x] 审计日志骨架（`audit_logs` 表、Repository、HTTP API、关键写操作埋点）
  - [x] 审计分页与服务端过滤（actor/action/resource_type/resource_id + page_size/page_token + total）
  - [x] 审计 before/after、trace_id、失败结果基础（`audit_logs` 扩展 before/after/trace_id/result/failure_reason；API/Web 展示；无外键）
  - [x] 审计导出治理基础（`GET /api/v1/audit-logs:export?format=json`，复用过滤条件、`audit:read` 权限、500 行上限、JSON envelope、Web 导出入口；CSV/脱敏策略后续增强）
- [x] Web UI 危险操作二次确认、权限感知操作（统一 `GuardedButton` / `PermissionGate`；用户/脚本删除与状态变更、任务触发、工作流运行/人工推进等按 RBAC 隐藏或二次确认）
  - [x] Web UI 审计日志查询页面（按操作类型筛选）
- [x] WASM 沙箱处理器边界（066：`WasmProcessorSpec`/`WasmResourcePolicy`/`WasmCapabilities` 稳定 worker 合约；选型 Wasmtime 45.x；默认拒绝网络与文件系统预打开）
  - [x] Worker 侧 Wasmtime 执行器基础（067：`tikeo-wasm` crate；fuel/epoch interruption、memory cap、无 WASI ambient imports、策略拒绝测试、最小 WAT smoke）
  - [x] WASM 脚本绑定与分发元数据基础（068：`DispatchTask.processor_binding` / `WasmProcessorBinding`；仅 `script:<id>` 且已审批、策略安全的 `language=wasm` 脚本下发模块与资源策略；Server 仅传递元数据不执行用户代码）
  - [x] WASM SDK 执行适配（069：Rust Worker SDK 在显式 `wasm` feature 下用独立 Wasmtime 适配器执行 `processor_binding.wasm`，未启用 feature 时返回清晰失败；Java SDK 对暂不支持的 WASM binding 显式失败且不调用普通处理器）
  - [x] WASM 分发完整性与策略可视化基础（070：`WasmProcessorBinding` 增加 version_id/version_number/module_sha256/module_signature；脚本版本快照保存 SHA-256；Rust SDK 校验模块摘要；Web 脚本页展示摘要与默认沙箱策略；Java Gradle protobuf plugin 升级并清除 Gradle 10 multi-string deprecation）
- [x] 多语言动态脚本处理器（Python/JavaScript/TypeScript/Shell/PowerShell/Rhai）
  - [x] 脚本定义 Storage / Migration / Repository / HTTP CRUD API / OpenAPI
  - [x] Web 脚本管理页面（列表、创建、审批、启用/禁用、删除）
  - [x] 脚本版本历史表（`script_versions`），创建和更新时产生不可变版本快照
  - [x] 版本 diff 对比 API（`GET /api/v1/scripts/{id}/diff?v1=&v2=`）与 Web 侧 diff 视图
  - [x] 发布指针、回滚 API 与 Worker 侧执行版本绑定（071：`scripts.released_version_id/released_version_number` 软关联不可变 `script_versions`；`POST /api/v1/scripts/{id}/publish|rollback` 更新发布指针并审计；WASM dispatch fail-closed，必须使用 released snapshot bytes/SHA-256/version metadata）
  - [x] 完整审批流状态机（多级审批、签名、生产发布门禁；127-130/Phase4 P1：release gate、本地签名 verifier、verified grants 与 Worker runtime grant enforcement 已闭环；外部 KMS/PKI 为后续增强）
  - [x] 发布/回滚策略门禁基础（087：publish/rollback 对历史危险 policy snapshot 再执行默认拒绝校验，阻断需要 URL/File/Secret grant 的版本并写入失败审计）
  - [x] 审批/签名元数据 fail-closed 基础（093：`ScriptReleaseRequest.approval_ticket/signature` 显式建模；未接真实签名验证前提供即拒绝并写入 `script_signature_verification_required` 审计）
  - [x] 脚本编辑器语法高亮（CodeMirror 6 Shell/Python/Node）
  - [x] Worker 侧语言 Runner 抽象（072：Rust SDK `ScriptRunner` / `ScriptRunnerTask` / `ScriptRunnerPolicy`，Shell/Python/JavaScript/TypeScript/PowerShell/Rhai 类型识别；`script` 表示统一脚本 Worker 能力，语言由 binding 传递且不等同于后端类型，默认后端为 sandbox=auto 自适应）
  - [x] Worker 侧沙箱执行器首个实现（073：Rust SDK `LocalSubprocessScriptRunner`，显式 opt-in，本地 stdin 子进程边界；校验 released version_id/version_number、content SHA-256、默认拒绝网络/文件/Secret，支持 timeout/output cap/runtime missing 测试；作为非默认兼容后端保留）
  - [x] 普通脚本 Worker Tunnel 协议绑定与语言能力路由（074：`ScriptProcessorBinding` 传递 released `script_versions` 快照 bytes/SHA-256/version/policy；dispatcher fail-closed 并按统一 `script` 能力过滤 worker，兼容旧 `script:<language>`、`script:*`、`*` 普通脚本 worker，直接 WASM 模块仍使用 `script:wasm`；Worker 必须显式注册对应语言 Runner；Web 展示语言能力与默认 WASM 后端语义，创建/编辑脚本语言仅提供普通脚本语言而不提供原始 WASM）
  - [x] Worker 侧容器化脚本 Runner 基础（075：Rust SDK `ContainerScriptRunner`，显式 opt-in，通过 Docker-compatible CLI `run --rm -i` 以 stdin 传入 released snapshot；默认 `--network=none`、`--read-only`、无宿主路径挂载，仅注入白名单 env 和 tikeo 元数据；单元测试覆盖命令边界与危险策略预检，真实 Docker/K8s 执行治理后续增强）
  - [x] 脚本执行治理失败可见性基础（077：dispatcher/Worker result 将无匹配 capability、缺 runner、策略拒绝、digest mismatch、timeout、output limit、runtime unavailable 归类为 `script_execution_governance` 实例日志；补充脚本 Worker Pool Docker/K8s 部署约束；Server 仍只调度不执行用户代码）
  - [x] 脚本执行治理查询与 UI 高亮基础（078：实例日志 DTO 解析 `script_execution_governance` JSON 为 event/failure_class/message 字段；`page_token=script_execution_governance` 可筛选治理日志；Web Instances 日志抽屉高亮治理失败；AlertCondition 增加 `script_governance_failure` 条件）
  - [x] 脚本执行治理审计落库基础（079：dispatcher 与 Worker result 路径将 `script_execution_governance` 失败同步写入 `audit_logs`，`resource_type=script_execution_governance` 软关联 instance id；审计 API/Web 支持 `failure_reason` 过滤；无外键）
  - [x] Java Worker 本地开发脚本 runner 修正与 SDK 侧沙箱工具管理（125：`sandbox.backend=auto` 对 Shell/Python/PowerShell/Rhai 解析为 srt/native-script 语义，对 JavaScript/TypeScript 解析为 Deno/V8 语义；Java 原生 SDK 提供 `net.tikeo.sandbox` 统一管理 Wasmtime/WasmEdge/Deno/V8/Rhai 工具解析与安装，Spring Starter 仅映射配置；避免真实脚本误走受限 bundled WASI shell runner。Python 源码若要走 WASM 需 Pyodide/CPython-WASI runtime，不等同于 Wasmtime 直接执行 `.py`）
- [x] 脚本策略引擎（能力声明、审批、资源限制、网络/文件策略）
  - [x] 默认拒绝策略元数据与不可变快照（072：`ScriptExecutionPolicy` 覆盖 resources/network/filesystem/secrets/env；`scripts.policy_json` 和 `script_versions.policy_json` 保存策略快照；HTTP create/update 拒绝网络/文件/Secret 危险能力；Web 可编辑资源/env 白名单并展示策略 diff）
  - [x] 策略审批、签名、URL/File/Secret grant 与生产发布门禁（本地 env-secret verifier 闭环；外部 KMS/PKI 为后续增强）
  - [x] 策略门禁失败审计基础（087：`failure_reason=script_policy_approval_required` 可查询 blocked publish/rollback，无外键）
  - [x] 签名验证缺失显式审计基础（093：`failure_reason=script_signature_verification_required` 可查询未验证 approval/signature 元数据的 blocked release）
- [x] 告警系统 (邮件/Slack/钉钉/飞书/企业微信/PagerDuty)
  - [x] AlertRule / AlertCondition / AlertDispatcher 安全 Webhook 通知骨架
  - [x] 告警规则 API、事件接入、去重静默、通知历史、恢复通知（080-082：alert_rules / alert_events 存储、HTTP API、script governance 事件历史 materialization、recovery 事件 append、alert-events:summary 运维汇总）
  - [x] 通知通道投递状态基础（091：`GET /api/v1/alert-rules/{id}/delivery-status` 本地解析 webhook/email/Slack/钉钉/飞书/企微/PagerDuty channel readiness，脱敏 target/secret）
  - [x] Webhook 真实投递基础（105：默认生产策略仅允许 HTTPS/public webhook；显式本地策略允许 loopback HTTP smoke；AlertDispatcher 返回脱敏投递结果；脚本治理 firing 事件会触发 channel delivery）
  - [x] 常见非 Webhook Provider 投递基础（107：Slack、钉钉、飞书、企业微信、PagerDuty adapter 生成 provider-specific JSON 并复用生产安全 URL 策略；本地 loopback smoke 覆盖 payload shape）
  - [x] 告警投递尝试历史基础（108：`alert_delivery_attempts` 无外键记录 event/rule/provider/脱敏 target/status/error/retry_state/next_retry_at；`GET /api/v1/alert-delivery-attempts` 支持 event/rule/provider/retry_state 过滤；script governance firing 投递结果持久化）
  - [x] Email/SMTP 本地投递基础（110：Email channel 支持 recipients/smtp_url/from；显式 local policy 下可向 loopback `smtp://` 投递纯文本邮件；默认缺少 SMTP 或非 loopback 策略 fail-closed；delivery-status 要求收件人与 SMTP endpoint）
  - [x] 告警 retry/backoff/DLQ 处理基础（111：`retry_pending` attempts 可按 `next_retry_at` 扫描；匹配当前 rule channel 后追加 retry attempt；旧 attempt 标记 `retry_consumed`，耗尽/缺失/无匹配进入 `dead_letter`；`POST /api/v1/alert-delivery-attempts:retry-due` 返回处理汇总）
  - [x] 告警 retry 后台调度（112：`alert_retry` 配置控制 bounded retry worker；server 启动时并行运行；按 cluster `can_schedule` 做所有权门控，避免 Raft follower 处理共享 retry 状态）
  - [x] 租户 namespace/app/worker-pool 管理 API 基础（113：新增 `worker_pools` 软链接元数据；`/api/v1/namespaces`、`/api/v1/apps`、`/api/v1/worker-pools` 支持鉴权 create/list；RBAC seed 增加 `tenants` read/manage；OpenAPI 覆盖）
  - [x] 租户 namespace/app/worker-pool Web 管理 UI（114：新增 `/scopes` 菜单与路由；支持 namespace/app/Worker Pool create/list；创建入口按 `tenants:manage` 守卫）
  - [x] 租户 scope 删除生命周期策略（115：namespace/app/Worker Pool DELETE 路由与 UI 删除入口；namespace/app 非空时拒绝删除，避免无外键软关联误级联；Worker Pool 元数据删除不影响在线 Worker）
- [x] Prometheus 指标 + Grafana Dashboard 模板
  - [x] Prometheus 指标端点（`/metrics`）与 HTTP/Worker 最小指标
  - [x] Metrics Summary API 基础（083：`GET /api/v1/metrics/summary` 汇总 worker online、实例状态、告警事件与脚本治理失败计数）
  - [x] Grafana Dashboard 模板基础（088：`observability/grafana/tikeo-phase3-dashboard.json` 覆盖 HTTP request/rate/latency、worker connected/dispatch 与错误率 SLO 占位查询，含本地 JSON 结构/指标引用测试）
  - [x] 调度队列 SLO 摘要基础（089：`GET /api/v1/metrics/summary` 增加 dispatch queue total/by_status/pending/running/oldest/average pending age，本地计算无需外部 Prometheus）
  - [x] 调度队列 pending-age Prometheus histogram 基础（096：`/api/v1/metrics/summary` 采样 dispatch queue SLO 后写入 `/metrics` 暴露的 `tikeo_dispatch_queue_pending_age_seconds{stat="oldest|average"}` 与 pending/running gauges）
  - [x] 实例/告警/治理 SLO Prometheus snapshot 基础（097：`/api/v1/metrics/summary` 同步写入 worker online、job instance status、success ratio、alert status、script governance failure gauges）
  - [x] Workflow / Map shard SLA Prometheus snapshot 基础（106：metrics summary 返回 workflow instance/shard status、success ratio、duration；`/metrics` 暴露 `tikeo_workflow_instance_duration_seconds` 与 `tikeo_workflow_shard_duration_seconds` histogram；Grafana 模板引用真实查询）
  - [x] 端到端 dispatch latency Prometheus snapshot 基础（109：`DispatchQueueSloSummary` 返回 completed_dispatches/average/longest dispatch latency；`/metrics` 暴露 `tikeo_dispatch_queue_dispatch_latency_seconds` histogram 与 completed gauge；Grafana 模板引用真实查询）
  - [x] 完整业务 SLO 指标（132：Prometheus recording rules、scrape config、Compose observability profile、Grafana recording-series 查询与 runbook；真实外部 Prometheus 仍按部署环境执行）
- [x] OpenTelemetry 分布式追踪
  - [x] HTTP Trace ID 传播基础（084：`x-request-id` / `x-trace-id` / W3C `traceparent` 解析，缺失时生成 `trc-*`，响应回写 `x-trace-id`，本地 tracing span 不依赖外部 collector）
  - [x] OTLP exporter 配置与状态基础（090：`observability.tracing` 配置、`GET /api/v1/observability/status` 脱敏显示 exporter/endpoint/header readiness）
  - [x] 真实 OTLP HTTP exporter 初始化与本地 collector smoke（119：server startup 根据配置启用 `tracing-opentelemetry` + OTLP/HTTP protobuf exporter；测试接收非空 `/v1/traces` payload 并验证配置 header 送达）
- [x] Java Spring Boot Starter SDK（优先）
  - [x] Gradle 多模块骨架：`tikeo` / `tikeo-spring` / `tikeo-spring-boot-starter`（JDK 21+；已替换 Maven 骨架）
  - [x] `@TikeoProcessor` 注解扫描与 auto-configuration 骨架
  - [x] Java gRPC Worker Tunnel 真实连接与心跳
  - [x] Java Spring Worker Demo 可持续运行与本地可见性（101：demo 不再启动后立即 close；支持 `TIKEO_WORKER_DRY_RUN=false` 连接 `127.0.0.1:9998`，可在 Worker 集群页面看到 java/spring-boot capability）
  - [x] Spring Boot Starter 生命周期闭环（120：`TikeoWorkerLifecycle` SmartLifecycle 自动 start/stop Worker client；支持 `tikeo.worker.enabled=false` 与 `auto-startup=false`）
- [x] Java Core SDK
- [x] Worker processor binding model（Job 定义与 Workflow job/map 节点支持 `processor_name`，Worker dispatch 按 processor name 路由，legacy 数据回退 `job_id`）
- [x] SDK 目录规范迁移：Rust SDK -> `sdks/rust/tikeo`，Java SDK -> Gradle/JDK21+，新增 `examples/<language>/<demo-name>` demo 骨架，并补齐 Rust / Java 可独立运行 demo 基础
- [x] Rust/Go SDK 与 demo 对齐 Java 手动联调能力（2026-06-04：默认 live 连接、结构化 scope/processor/capability、script runners 对齐 Java sandbox 名称、任务日志持久化、重连循环和手动验收数据入库）

#### Phase 3 closeout notes (2026-05-23)

Phase 3 closeout 状态已在 2026-05-28 复核：原先保留未勾选的 OIDC opaque session、真实 TLS/mTLS listener、脚本发布门禁/签名/grant、生产告警投递硬化、Prometheus/Grafana recording-rule 校验、Worker 生命周期治理、Java/Rust management SDK API-Key 闭环等均已在后续 Phase 3 closeout / Phase 4 P0-P1/P2 服务可用性切片中闭环，并按“本地可验证 foundation + 外部系统增强后置”的标准标记完成。外部 KMS/PKI、Go ergonomic run-loop、Python/Node SDK ergonomics 仍保留在 Phase 4 对应 P1/P2；Helm/K8s production baseline 已在后续部署切片闭环；PowerJob/XXL-JOB 等迁移工具统一降为最低优先级 backlog，不再算 Phase 3/4 P0-P2 的服务可用性缺口。

#### Phase 3/4 service-usage priority rebalance (2026-05-24)

剩余工作按“是否直接影响真实团队把 tikeo 作为共享服务使用”重新排序。原则：先补齐登录、安全传输、Worker 生命周期、部署运维和可靠告警这些服务可用闭环；再做生产治理增强；最后做生态集成和高级差异化。各类迁移工具（PowerJob、XXL-JOB、后续同类平台导入器）不再参与 P0-P2 排序，统一放入最低优先级迁移 backlog，等核心服务体验稳定后再做。

**P0 — 服务使用 / 生产上线阻塞项（优先实现）**

- [x] OIDC 外部 subject → 本地 user/role/tenant 映射，并签发 tikeo opaque session（122：provider token 不成为本地登录态；`auth_sessions` + moka 仍是唯一登录态来源；OIDC scope binding 可限制 namespace/app/worker_pool）。
- [x] 真实 HTTP 与 Worker Tunnel TLS/mTLS listener、证书 reload/rotation、启动诊断和失败回滚（123：HTTP 新连接重载证书；Worker Tunnel 启动加载 TLS/mTLS；启动与 `/security/transport` 均 fail-closed 报告证书配置错误）。
- [x] Worker 身份与会话生命周期治理（K8s/Docker 与裸机/VM/systemd 同等支持；Logical Worker / Session / generation / fencing token / lost reason 分层）。
  - [x] Slice A 内存态 session generation/fencing 基础（124：`WorkerRegistered` 返回 generation/fencing_token；Heartbeat 携带并校验；同 logical key 重注册会将旧 session 标记为 `replaced_by_new_generation`，调度与 `/workers` 只使用最新 online generation；Rust/Java SDK 已对齐 heartbeat fencing 字段）。
  - [x] Slice B 持久化 `worker_logical_instances` / `worker_sessions` / `worker_session_events`，注册/替换/心跳写入持久层，启动迁移兼容旧 SQLite 库。
  - [x] Slice C lease scanner：过期 online session 持久标记为 `offline / lease_expired_unknown`，写入 `lease_expired` 事件，不将 heartbeat timeout 误判为 crash。
  - [x] Slice D graceful unregister：协议新增 `UnregisterWorker`，Server/Rust SDK/Java SDK 支持主动下线并标记 `stopped / graceful_shutdown`。
  - [x] Slice E assignment token 校验：dispatch 下发 assignment token，Rust/Java SDK 回传，Server 拒绝缺失/错误 token 的日志与结果。
  - [x] Slice F Web lifecycle history UI：`/workers/history` 返回持久 sessions/events，Worker 集群页面按在线/异常/历史分层显示。
  - [x] Slice G Worker 可见性快照持久化（2026-06-04：`worker_sessions` 持久保存 capabilities/structuredCapabilities/labels/master 快照；`/api/v1/workers` 合并 live registry 与 DB online sessions，server 重启后可先展示持久在线快照，不能再只依赖内存注册表）。
- [x] 部署与运维 bootstrap：本地/裸机/systemd/Compose 与 Kubernetes Helm baseline 已落地（125/135/150：Compose env defaults、systemd server/worker unit/env、Worker identity env、裸机 config smoke helper、readyz + worker dry-run smoke、Helm 外部 DB Secret、HTTP/Worker Tunnel TLS/mTLS Secret、Ingress、探针、资源参数、PodDisruptionBudget、NetworkPolicy、ServiceMonitor、Gateway API GRPCRoute、values.schema.json 与 rollback runbook 已落地）。
- [x] 生产告警投递硬化：SMTP TLS/auth/secret reference、Provider secret 管理、重试/DLQ 可视化与最小 live smoke（126：Email 支持 smtps/smtp+starttls、AUTH LOGIN、env secret refs；新增 retry/DLQ queue-status API 与 Web 告警投递页；保留 loopback SMTP smoke）。

**P1 — 生产治理增强 / 常见企业用法**

- [x] 完整脚本审批/签名/KMS 与 URL/File/Secret grant，生产发布门禁闭环。
  - [x] 发布门禁只读预检基础：`GET /api/v1/scripts/{id}/release-gate` 返回版本是否可发布、阻断原因、所需动作，并明确真实签名验证尚未启用。
  - [x] 本地签名验证边界：`script_governance.release_signature_secret_ref` 默认关闭；配置 `env:` secret 后，发布/回滚要求 approval ticket 与绑定 script/version/content digest 的 `sha256:<hex>` 签名匹配。
  - [x] 成功签名发布元数据持久化与展示：发布指针保存 approval ticket、签名、校验时间与校验人，并在 HTTP `ScriptSummary` 和 Web Scripts 页面展示。
  - [x] URL/File/Secret grant 载荷边界：`ScriptReleaseRequest.grants` 显式建模 `url/file_read/file_write/secret`，当前任何非空 grant 都 fail-closed，直到接入 verified grant enforcement。
  - [x] Verified grant 证据持久化边界：发布指针可保存 verified grant JSON、校验时间与校验人，并通过 `ScriptSummary`/Web 展示。
  - [x] 本地 signed grants 闭环：配置 `script_governance.release_signature_secret_ref` 后，签名 payload 绑定 grants JSON，验证通过才移动发布指针并持久化 `release_grants` 证据；未配置时 grants 仍 fail-closed。
  - [x] Worker runtime grant enforcement 闭环：Worker Tunnel `ScriptProcessorBinding` 携带 signed URL/File/Secret grant；Rust SDK policy 显式接收 `allowed_network_hosts`/文件/secret refs，Local runner 对 grant fail-closed，Container runner 仅将文件 grant 转成显式 bind mount，网络/secret grant 无安全 runtime provider 时继续 fail-closed；Java SDK proto/测试同步覆盖 grant-bearing script binding 且仍不执行脚本。
- [x] OIDC tenant/app/role 绑定策略与高级租户隔离 UI（131：`/api/v1/oidc-identities` 管理 issuer+subject -> local user + namespace/app/worker-pool scope；OIDC callback 未映射 fail-closed；Scopes 页面可管理映射）。
- [x] Prometheus/Grafana recording-rule 校验、运维 runbook 与真实 scrape 验证（132：本地 Compose Prometheus profile + committed recording rules/config/runbook；CI 覆盖规则/仪表盘引用一致性）。
- [x] Go SDK（2026-06-04：official gRPC/protobuf Worker Tunnel run-loop、默认 live demo、结构化 processor/script capability、assignment token 任务日志与重连循环已落地；README 已说明 protoc 与 Dockerfile 安装方式）。
- [ ] Python SDK（用户要求先延期）。
- [ ] Node.js SDK（用户要求先延期）。


#### Source file hygiene checkpoint (2026-05-25)

后续源码文件必须保持单文件 `<=1500` 行；`mod.rs` / `lib.rs` 等入口文件只做模块声明和 re-export，不堆实现或测试。2026-06-05 迁移专项已保证本轮触达文件低于 1500 行，但全仓库重新审计发现仍有历史超限文件（如 dispatcher、repository、workflow、Web i18n/API client 等），后续不得再宣称全仓库已满足该规则，需优先拆分或建立明确生成文件豁免边界。

**P2 — 生态接入 / 高级差异化（不阻塞服务先跑起来）**

- [x] SDK Management API-Key 签发与鉴权方案（2026-05-31 已升级为长期 Service Account 方案）：SDK 端 management client 不走人工 session token，也不收敛到某个用户账号的 RBAC 权限；Server 提供 `POST/GET/PATCH/DELETE /api/v1/management/service-accounts` 独立维护 app-scoped 机器身份，API-Key 创建时只能选择已有 active Service Account，禁用 Service Account 会吊销其 active API-Key；`POST/GET/PATCH/DELETE /api/v1/management/api-keys` 继续负责凭证签发/元数据编辑/吊销，`tk-` + 64 位大小写字母数字 CSPRNG/rejection sampling 生成，服务端只存 hash 与两端明文脱敏值；`X-Tikeo-API-Key` 鉴权映射为 `sdk_api_key/app_service` principal，并以 Service Account 当前 active 状态与 namespace/app scope 作为最终授权边界；Web `/api-keys` 支持 Service Account 管理、选择已有身份签发 Key、一次性明文展示、编辑名称/权限/有效期、吊销与 last-used 观测；Java 原生 SDK、Spring Boot Starter、Rust SDK management client 与 Java demo 均已接入 API-Key。验证：`cargo test -p tikeo-server sdk_api_key -- --nocapture`、`cargo test -p tikeo-server disabling_service_account_revokes_bound_sdk_keys -- --nocapture`、`cd web && bun run typecheck && bun test --run src/api/client.test.ts`、Java SDK/starter/demo 相关测试。
- [x] GitOps/IaC Manifest 导出与 drift diff（2026-05-29：`GET /api/v1/gitops/manifest`、`POST /api/v1/gitops/diff`、Web GitOps/IaC 页面、manifest/CRD/Terraform contract 样例已落地）。
- [x] Terraform Provider 与 K8s CRD controller/operator 已补齐（2026-05-30：`deploy/terraform/provider` 提供真实 Terraform Plugin Framework provider，含 `tikeo_manifest` data source 与 `tikeo_manifest_diff` resource；`deploy/k8s/operator` 提供 `TikeoManifest` CRD reconciler/operator CLI，按 `/api/v1/gitops/diff` 写入 status evidence；CRD 增加 status subresource、conditions、checksum、summary/lastDiff）。
- [x] 任务版本管理与回滚（2026-05-28：`job_versions` 不可变快照表、创建/编辑/回滚自动追加版本、`GET /api/v1/jobs/{job}/versions`、`POST /api/v1/jobs/{job}/rollback`、Jobs 页面版本历史抽屉与回滚入口已落地；回滚生成新的最新版本，不覆盖历史）。
- [x] 任务依赖自动发现、拓扑图形画布、跨工作流影响分析与回放基础（2026-05-28：已从 Job + Workflow definition 自动推导 job/workflow 节点、workflow_job_ref / workflow_job_dependency 边与 unresolved 缺失引用；`GET /api/v1/jobs/topology` 返回 layer/position 供画布渲染；新增 `GET /api/v1/jobs/{job}/impact` 汇总引用工作流、上游/下游 Job 与风险摘要；新增 `GET /api/v1/workflow-instances/{id}/replay` 返回 instance + definition + events + graph replay bundle；Jobs 页面“任务拓扑”入口跳转到 `/jobs/topology` 二级页面，二级页面承载 SVG 图形画布并可点击 Job 查看跨工作流影响分析）。
- [x] 高级 Webhook/事件源基础（2026-05-28：入站事件源 `POST /api/v1/events/webhooks/{job}:trigger` 已落地，复用 `instances:execute` 与 namespace/app scope 鉴权，创建 `webhook` trigger instance 并记录 `webhook_event_source` payload 日志；GitHub/GitLab/Alertmanager 等 provider 适配器保留后续增强）。
- [x] 任务灰度发布基础（2026-05-28：Job 增加 canary target/percent，显式 UI/API trigger 按百分比路由到 canary Job，并在 `JobInstanceSummary.canaryRouting` 返回 original/routed job；Jobs 页面支持配置灰度目标/比例并在触发后提示命中灰度。自动回滚、worker tag 灰度和指标门禁仍保留后续增强）。
- [x] 插件系统。


#### Worker 服务集群 Master 选举补充（2026-05-30）

Worker 集群与 tikeo server 集群都必须具备 master 选举能力。Server 侧由 raft-rs 负责调度所有权；Worker 侧新增结构化 `WorkerClusterElection` 注册声明，默认启用，选举域为 `namespace/app/cluster/region`（可显式覆盖 domain），并使用 priority + worker_id 做确定性唯一 master 选择。Server `WorkerRegistry` 在注册、心跳、transport error、unregister/replacement 后重新计算 domain master，向 Worker 列表 API 暴露 `master.domain/isMaster/masterWorkerId/term/fencingToken`。普通 single dispatch 使用 `find_ordered_dispatch_workers` 优先派发到对应 domain master，broadcast 仍按 selector fan-out 到所有匹配 session。Worker session generation/fencing token 继续保护重连、旧连接和任务 assignment，worker master fencing token 用于观测与后续 worker-side 协同扩展，不再依赖 `plugin-processor:<type>` 这类字符串约定表达集群角色。

### Phase 4: 高级能力 (月 10-12)

**目标**：超越 PowerJob，建立差异化竞争力。

**P0 — 服务使用 / 运维优先**

- [x] Worker 身份与会话生命周期治理（Worker Pool / Logical Worker / Worker Session 三层身份；兼容 K8s/Docker 与裸机/VM/systemd；generation + fencing token；graceful/replaced/heartbeat_timeout/transport_error 证据分级；历史归档与 Worker UI 分层，详见 `design/worker-identity-lifecycle-design.md`）
- [x] 部署与运维 bootstrap（Compose/systemd/裸机模板与 K8s Helm Chart baseline；包含 Worker identity env、systemd worker 模板、readyz/Worker dry-run smoke、外部 DB Secret、TLS/mTLS Secret、Ingress、探针、资源参数、PodDisruptionBudget、NetworkPolicy、ServiceMonitor、Gateway API GRPCRoute、values.schema.json 与 rollback runbook）
- [x] 多租户隔离增强（tenant/app/worker-pool scope policy 与 OIDC tenant binding 对齐；131：OIDC identity mapping API/UI 复用 scope binding，未映射 external subject 不签发本地 session）

**P1 — 常见接入与生产治理**

- [x] Go SDK（2026-06-04：official gRPC/protobuf Worker Tunnel run-loop、默认 live demo、结构化 processor/script capability、assignment token 任务日志与重连循环已落地）
- [ ] Python SDK（用户要求先延期）
- [ ] Node.js SDK（用户要求先延期）
- [x] 脚本生产治理增强（完整审批/签名/KMS、URL/File/Secret grant、生产发布门禁；本地 env-secret verifier 闭环，外部 KMS/PKI 后续增强）
- [x] Prometheus/Grafana recording-rule 与真实 scrape 验证（132：规则、Prometheus scrape config、Grafana recording-query 与 runbook）。

**P2 — 生态接入与高级差异化**

- [x] SDK Management API-Key 签发与鉴权方案（2026-05-31 已升级为长期 Service Account 方案）：SDK management client 使用 app-scoped API-Key，不使用人工 session token，不绑定某个用户账号的 RBAC 权限；后台管理员先维护 Service Account 机器身份，再针对已有 active Service Account 手动签发、编辑元数据、吊销授权，供 Java/Rust SDK 管理任务、触发任务和读取实例状态。

  **已实现约束 / 后续增强边界**：
  - Key 明文格式固定为 `tk-${64位大小写字母数字}`，即前缀 `tk-` + 64 个 `[A-Za-z0-9]` 字符；全局唯一，只在创建/轮换时返回一次明文。
  - 生成算法采用业界通用 CSPRNG API-key 方案：使用 OS 级密码学安全随机源（Rust `OsRng` / Java `SecureRandom` / WebCrypto 等同等级来源），对 62 字符 alphabet 做 rejection sampling/无模偏采样，生成 64 位 base62 随机串；约 330 bit 熵，不使用 UUID、时间戳、递增序列或可预测 PRNG。
  - Service Account 是一等资源：`service_accounts` 存储稳定机器身份 id、名称、description、namespace/app、可选 worker_pool、active/disabled 状态与创建/更新人；API-Key 只保存绑定的 `service_account_id` 与名称快照，创建时必须选择已有 active Service Account，禁用 Service Account 必须吊销其关联 active Key。
  - API-Key 存储只保存 `key_id`、`prefix`、HMAC-SHA256/SHA-256 hash（建议带 server pepper）、app scope、授权 scope、状态、过期时间、last_used_at、created_by/revoked_by/rotated_from 与审计证据；禁止持久化明文 key。
  - 鉴权边界是 app-scoped service credential + 当前 Service Account 状态：请求通过 `X-Tikeo-API-Key: tk-...`（或 SDK 内部等价 header）进入 management API；认证后 principal 类型标记为 `sdk_api_key` / `app_service`，不能伪装成人类用户 session，也不能复用用户 RBAC role expansion；若绑定 Service Account 被禁用、迁移 scope 或不存在，鉴权/授权必须 fail-closed。
  - 授权模型为后台针对 namespace/app 手动签发的 allow-list：可细分 `jobs:read/create/update/trigger`、`instances:read/logs:read`、`workflows:*` 等 SDK management scopes，并强制落在签发时绑定的 namespace/app 内；越权访问其它 app 必须 fail-closed。
  - SDK 侧配置应从 `tikeo.management.api-key` / `TIKEO_MANAGEMENT_API_KEY` 读取，替换当前 `token` 语义；Java/Rust SDK 都要同等支持，Spring Boot Starter 只做配置映射，不拥有鉴权逻辑。
  - 后台已提供管理员 API/UI：Service Account 创建/列表/编辑/禁用；API-Key 创建、列表（两端明文中间脱敏）、编辑名称/授权 scope/有效期、吊销、last-used 观测与审计日志；所有身份维护与凭证签发/编辑/吊销操作仍由后台人工 session + RBAC 保护。轮换曾评估但按当前产品决策改为“编辑不改 key”，需要换 key 时新建后吊销旧 key。
  - 与已有 API Token/OIDC session 明确分层：API Token 是用户权限收窄后的 bearer；OIDC 只换本地 opaque session；SDK Management API-Key 是 app 级服务凭据，不能被用户 token/RBAC 自动推导生成。

- [x] GitOps/IaC Manifest 导出与 drift diff
- [x] Terraform Provider 与 K8s CRD controller/operator 已补齐（2026-05-30：Terraform Provider 具备 provider schema、manifest data source、manifest diff resource；K8s operator 具备 CRD status、diff reconciler、RBAC/sample/CLI。批量 apply 仍不绕过 typed CRUD/RBAC/审批/审计链）
- [x] 任务依赖自动发现、拓扑图形画布、跨工作流影响分析与回放基础（2026-05-28：`GET /api/v1/jobs/topology`、`GET /api/v1/jobs/{job}/impact`、`GET /api/v1/workflow-instances/{id}/replay` 已落地；Jobs 页面拓扑入口已改为 `/jobs/topology` 二级页面，二级页面承载 SVG 图形画布、全屏/退出全屏切换、依赖边/引用边/unresolved 引用列表，并支持选中 Job 查看跨工作流影响分析；Replay API 先作为事故复盘 bundle 暴露，后续可接入 Workflow 实例详情页做时间轴播放）。
- [x] 智能调度建议基础（2026-05-28：新增 `GET /api/v1/jobs/{job}/scheduling-advice`，基于 Job processor/script 绑定推导 required capability，结合在线 Worker 能力与最近实例失败数返回 ready/severity/reason/eligibleWorkers；Jobs 页面增加“调度建议”抽屉。完整历史耗时/资源预测仍保留后续增强）
- [x] 插件系统 (自定义处理器类型、告警通道)（2026-05-28：新增 `plugins` 注册表与 `GET/POST/PATCH/DELETE /api/v1/plugins`；插件声明 `processorTypes` 与 `alertChannelTypes`，Job 增加 `processorType` 并按 `plugin-processor:<type>` 能力匹配 Worker；告警规则支持插件 channel type readiness 与 webhook 模板投递；Web 增加 `/plugins` 插件系统页面，Jobs 创建/编辑可选择插件处理器；Java demo 与 Rust demo 可广告 `plugin-processor:sql`，本地 `tikeo-dev.db` 已注入 Ops Plugin 联调用例）。
- [x] Webhook 入站/出站基础（2026-05-28：出站告警 Webhook 已有；新增入站 `POST /api/v1/events/webhooks/{job}:trigger`，支持外部系统以 session/API Token/SDK API-Key 触发 Job 并记录事件 payload。高级 provider 签名校验、模板映射、重放保护后续增强）
- [x] 任务版本管理与回滚（2026-05-28：`job_versions` 不可变快照、版本列表 API、回滚 API、Jobs 页面版本历史与回滚入口；验证 `cargo test -p tikeo-storage job_version -- --nocapture`、`cargo test -p tikeo-server job_version -- --nocapture`、Web lint/build/API/UI tests）。
- [x] 灰度发布基础（2026-05-28：`canaryJobId`/`canaryPercent`、显式 trigger canary routing、response `canaryRouting`、Jobs 页面配置与命中提示已落地；A/B 指标分析、按 worker tag 灰度、失败自动回滚后续增强）

**最低优先级 — 迁移工具 Backlog（核心服务体验稳定后再做）**

- [ ] PowerJob 迁移工具（从 Phase 3/4 P2 下调；与迁移报告/双跑能力一起实现）。
- [ ] XXL-JOB 迁移工具（从 Phase 3/4 P2 下调；覆盖 xxl_job_group / xxl_job_info / CRON / child_jobid / GLUE 脚本迁移报告）。

### 15.5 创新能力清单

这些能力不单独作为“附录”存在，而是贯穿调度、执行、工作流、安全和运维设计：

| 能力 | 说明 | 所属阶段 |
|------|------|----------|
| Worker 主动连接公共服务 | 无业务入站端口，Server/Worker 可分离部署到不同容器、namespace、集群、VPC；反向调用复用 Worker tunnel 穿透多级网络 | Phase 1-2 |
| Worker 身份、会话生命周期与服务集群 master 选举 | 已按 `design/worker-identity-lifecycle-design.md` 将 Worker Pool、Logical Worker Instance 和 Worker Session 分层；用 generation/fencing token 处理重启、替换、掉线、transport error 和历史归档；新增结构化 WorkerClusterElection 与 domain master 状态，single dispatch 优先 worker master，避免同一 worker 服务集群多节点抢占导致顺序混乱 | Phase 4 |
| GitOps/IaC | YAML、K8s CRD、Terraform Provider、PR diff、变更审计；Terraform Provider 与 K8s CRD controller/operator 已补齐 | Phase 4 |
| 任务版本与灰度 | Job version、canary、按 worker tag 灰度、失败自动回滚 | Phase 4 |
| 调度仿真 | 变更前模拟未来 N 次触发、misfire 结果、资源占用 | Phase 4 |
| 平台管理控制台 | 嵌入式 Web UI + HTTP/OpenAPI 管理接口，覆盖任务、实例、工作流、Worker、脚本、安全、审计和告警 | Phase 1-4 |
| 工作流回放 | 已提供 `GET /api/v1/workflow-instances/{id}/replay` 事故复盘 bundle（instance/definition/events/graph）；后续增强为实例详情页时间轴播放和状态逐帧动画 | Phase 4 |
| 智能调度 | 已在调度建议中提供完整历史耗时统计（avg/p50/p95/max、completed/failed）和基于 eligible Worker 标签的资源预测（预计耗时、推荐并发、CPU/Memory capacity）；后续可接入实时 Worker 负载和队列排队模型 | Phase 4 |
| 策略引擎 | OPA/Rego 或内置 DSL，控制 Shell/SQL/HTTP/生产变更审批 | Phase 3-4 |
| WASM 插件 | 语言无关、安全沙箱、插件签名与版本管理；同时作为普通脚本默认通用沙箱后端。当前插件注册中心已先闭环自定义处理器类型与自定义告警通道；WASM 插件包签名/版本隔离继续作为安全增强演进。 | Phase 3-4 |
| 多语言动态脚本 | Python/JavaScript/TypeScript/Shell/PowerShell/Rhai 等受控运行；`script` 为统一脚本 Worker 能力，具体语言在 binding 中传递；默认 `sandbox=auto`：可编译到 WASM 的内容优先 Wasmtime，原生命令/现成二进制优先 Anthropic Sandbox Runtime (srt)，JavaScript/TypeScript 逻辑优先 Deno，未匹配时回退 Wasmtime；可显式指定 wasmtime/wasmedge/srt/deno/v8/docker/podman/custom | Phase 3-4 |
| 事件驱动 | Webhook、Kafka/NATS/Redis Stream 触发源，出站 HMAC 回调 | Phase 4 |
| 多租户配额 | namespace/app/worker pool 级并发、QPS、日志量、存储 TTL | Phase 3 |

---

## 16. xxl-job / PowerJob 迁移指南 (概要)

为了降低既有用户迁移成本，tikeo 将同时提供 xxl-job 与 PowerJob 的迁移工具，但迁移策略不同：xxl-job 偏“补能力”，PowerJob 偏“替换架构债”。

### 16.1 从 xxl-job 迁移

可自动迁移：

1. `xxl_job_group` → tikeo app / worker pool。
2. `xxl_job_info` → tikeo job definition。
3. `CRON` / `FIX_RATE` → tikeo schedule。
4. `executor_handler` → processor name。
5. `executor_route_strategy` → worker selector / dispatch policy。
6. `child_jobid` → 简单 DAG workflow 边。

需要人工确认：

- GLUE/Shell/Python/JavaScript/TypeScript/PowerShell 等动态脚本任务迁移到 Script Processor，并补充语言运行时、沙箱策略、资源限制、网络/文件策略和审批策略。
- `child_jobid` 只能迁移为基础 DAG，复杂条件、上下文、补偿逻辑需要重新建模。
- Executor 本地日志无法保证完整迁移，只能迁移索引或归档文件。
- FIX_DELAY 如果只是文档使用预期，需要按业务语义重新创建。

### 16.2 从 PowerJob 迁移

可自动迁移：

1. App / Namespace / Job / Workflow 元数据。
2. TimeExpressionType：API、CRON、FIXED_RATE、FIXED_DELAY、WORKFLOW、DAILY_TIME_INTERVAL。
3. ExecuteType：STANDALONE、BROADCAST、MAP、MAP_REDUCE。
4. Workflow DAG：JOB、DECISION、NESTED_WORKFLOW。
5. 官方处理器参数：HTTP、Shell/Python、SQL、FileCleanup 等尽量映射。

需要人工确认：

- Groovy decision 脚本 → tikeo 安全表达式或 WASM 处理器。
- SQL Processor → 数据源白名单、参数化模板、dry-run 和审批策略。
- HTTP Processor → URL policy、内网地址阻断和签名配置。
- Dynamic JAR / External Processor → SDK Processor、WASM 或容器任务。
- Worker tag / protocol / external address → Worker Pool selector。

### 16.3 双跑与回滚

迁移工具应支持：

- `tikeo migrate --from xxl-job --db mysql://... --dry-run`
- `tikeo migrate --from xxl-job --db mysql://... --dry-run --report migration-report.json`
- `tikeo migrate --from powerjob --db mysql://... --dry-run`
- 生成迁移报告：不可迁移项、风险项、安全策略缺口、下次触发时间差异。
- 支持 xxl-job / PowerJob 与 tikeo 双跑一段时间，通过实例结果和日志对账后再切流。
- 迁移工具已从 Phase 3/4 P0-P2 下调为最低优先级 backlog；核心服务体验、SDK 与治理闭环稳定后再落地 PowerJob 与 XXL-JOB 迁移工具。XXL-JOB 迁移重点仍是基础任务、路由策略、child_jobid DAG 化和 GLUE 脚本风险报告。

---

## 17. 风险与应对

| 风险 | 影响 | 应对策略 |
|------|------|----------|
| Rust 开发者招聘难 | 开发速度 | 核心用 Rust，SDK 层可用各语言原生开发；文档驱动社区贡献 |
| WASM 生态不够成熟 | 处理器灵活性 | sandbox=auto 以 Wasmtime 作为可编译 WASM/直接 WASM 的高安全插件方案；原生 Shell/Python/PowerShell 脚本默认走 srt/native-script 语义，JavaScript/TypeScript 默认走 Deno；未部署 srt/Deno/V8/Docker/Podman/custom 时只允许 development-only 本地 runner 或明确失败，不再把真实 shell 脚本伪装进受限 WASI shell 微运行时 |
| Raft 实现复杂度 | 集群稳定性 | 使用 TiKV raft-rs (`raft`) 作为共识核心，项目只实现存储、transport、membership 和调度 fencing glue，避免自研共识 |
| 前端开发资源 | UI 体验 | 前端固定在 `./web`，使用 React + Ant Design + Bun，保持独立工程边界 |
| 与 PowerJob 功能差距 | 用户迁移意愿 | 严格对照表 + 迁移工具 + 兼容 API |

---

## 18. 总结

tikeo 的核心判断是：**xxl-job 的问题是能力不够，PowerJob 的问题是功能堆叠后架构债过重**。二者都能解决一部分“定时任务”问题，但都不适合作为企业平台中面向多团队、多语言、多集群、多租户的公共任务调度基础设施。

| 维度 | xxl-job 的思路 | PowerJob 的思路 | tikeo 的思路 |
|------|----------------|-----------------|----------------------|
| 通信 | Admin HTTP 反向调用 Executor | AKKA/HTTP/MU 多协议反向调用 Worker | Worker 主动 gRPC 双向流，单协议、单端口 |
| 调度 | DB 全局锁 + 秒级扫描 + 60 秒内存 ring | DB currentServer + 15s loop + 内存时间轮 + Worker 本地频繁任务 | 持久化 trigger_event + lease shard + near-time cache |
| 集群 | MySQL `FOR UPDATE` 串行化 | DB 锁 + PING 选举，非共识 | Raft / fencing token / 可验证租约 |
| 工作流 | child_jobid 触发子任务，不是真 DAG | DAG/Decision/NestedWorkflow，但依赖 Groovy 与数据库状态 | 事件溯源 DAG 状态机、安全表达式、强类型上下文 |
| 执行 | Java Executor + GLUE/脚本 | Java Worker + 官方处理器 + 外部 JAR | 多语言 SDK + WASM/子进程/HTTP/gRPC/SQL 安全执行 |
| 部署 | Spring Boot Admin + MySQL + Executor 端口 | Spring Boot + 多协议多端口 + H2/JPA | 单二进制、单端口、SQLite/MySQL/Pg/CRDB |
| K8s/Docker/跨集群适配 | 需要 Executor 可被访问 | 需要 Worker 可被访问，external address 配置复杂 | Server/Worker 容器可跨网络部署；业务 Pod/Worker 无入站暴露要求，反向调用经 tunnel 穿透 namespace/cluster/VPC/NAT |
| 安全 | 默认 token 与宿主脚本执行 | Groovy/SQL/HTTP/脚本多攻击面 | mTLS/RBAC/OIDC/审计/沙箱/策略引擎 |
| 可观测 | 本地日志 + 轮询 | 队列批量日志，可能丢 | OTLP/Prometheus/流式日志/trace/回放 |

**结论**：tikeo 不是对 PowerJob 或 xxl-job 的简单替代实现，而是面向企业平台的重新设计。最核心的价值不是“多几个调度类型”，而是把任务调度的通信、状态、安全、观测和多租户治理全部做成可长期演进的基础设施。
