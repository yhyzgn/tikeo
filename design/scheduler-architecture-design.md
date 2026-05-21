# scheduler — 下一代分布式任务调度平台架构设计

> **Rust 原生 | 单二进制 | gRPC 标准 | Cloud Native First | 零历史债**  
> 本设计保留合理的架构、协议、组件、部署、技术栈和路线图设计，项目名称统一为 **scheduler**，并增强 PowerJob / xxl-job 源码级剖析、K8s 公共服务化问题、全新开发必要性与创新功能点。

---

## 1. 项目概述

### 1.1 什么是 scheduler

scheduler 是一个用 Rust 从零构建的分布式任务调度与计算平台，目标是**完全覆盖 PowerJob 的全部功能特性**，同时在性能、易用性、部署体验和安全模型上实现质的飞跃。

### 1.2 为什么要从 0 开发

经过 xxl-job 与 PowerJob 源码调研后，结论明确：**xxl-job 是能力上限不足，PowerJob 是能力堆叠后架构债过重**。二者都不适合作为企业平台统一调度底座。

| 痛点 | xxl-job 现状 | PowerJob 现状 | scheduler 目标 |
|------|--------------|---------------|----------------|
| 调度能力 | 核心持久调度只有 CRON / FIX_RATE；FIX_DELAY 在源码枚举中仍是注释状态；任务依赖只是 child_jobid 串联 | 调度方式更多，但 CRON/工作流/秒级任务由多套路径实现，调度责任散落在 Server 与 Worker | 统一 Schedule / Trigger Event 模型，覆盖 CRON、FIX_RATE、FIX_DELAY、API、延迟、一次性、日历调度 |
| 执行模型 | 单机、分片广播为主，无 MapReduce 内核，无真正 DAG 工作流 | 有 STANDALONE / BROADCAST / MAP / MAP_REDUCE / DAG，但状态、通信和本地持久化耦合重 | 覆盖 PowerJob 执行模型，并把 MapReduce、DAG、长运行任务做成可恢复、可观测状态机 |
| 公共服务化 | Admin 反向访问 Executor，Executor 必须暴露入站端口 | Server 反向访问 Worker，Worker 也必须绑定端口并上报 external address | Worker 主动建立 gRPC/HTTP2 长连接隧道，Server 不回连业务 Pod，天然适配 K8s/Docker/NAT/多级网关/跨集群 |
| 集群协调 | MySQL `FOR UPDATE` 全局调度锁 | DB 锁 + `currentServer` + PING 选主，不是共识 | Raft / lease shard / fencing token，调度归属可验证、可恢复 |
| 部署体验 | Spring Boot Admin + MySQL + Executor 端口；能力简单但仍非单二进制 | Java 8+、Spring Boot、Undertow、Akka、Vert.x、MySQL、本地 H2、多端口 7700/10086/10010/10077 | Server/Worker 均容器优先，K8s/Docker/Compose/Nomad/systemd 全支持；单二进制、单端口、开发态 SQLite、生产态 MySQL/PostgreSQL/CockroachDB |
| 安全边界 | 默认 token、GLUE/Shell/Python/Node/PowerShell 在宿主执行 | Groovy 决策、SQL Processor、HTTP Processor、脚本下载执行、customQuery 等攻击面大 | 默认 mTLS/RBAC/OIDC/审计；WASM/子进程沙箱；URL policy；参数化 SQL |
| 可观测性 | Executor 本地日志，Admin 轮询读取 | Worker 内存队列批量上报，队列满或 Server 不可用会丢日志 | gRPC 流式日志、背压、OTLP、Prometheus、审计事件、事故回放 |
| 可维护性 | 架构简单但功能空间太窄 | Akka/HTTP/MU 三协议、JPA/H2、本地文件交换、历史兼容层多 | 协议、状态机、存储、执行沙箱从一开始按企业平台设计 |

因此 scheduler 不是“PowerJob 的 Rust 版本”，也不是“xxl-job 加功能”。它是面向企业平台公共调度服务的一次重新建模：**用更少的核心抽象承载更多、更可靠、更安全的能力**。
### 1.3 源码调研后的核心结论

本设计不再只以 PowerJob 为单一参照，而是把 xxl-job 与 PowerJob 都作为反例基线：

| 结论 | xxl-job 源码表现 | PowerJob 源码表现 | scheduler 设计取舍 |
|------|------------------|-------------------|--------------------|
| 公共服务化的最大阻碍是 Worker/Executor 入站可达 | Admin 通过 HTTP 调 Executor 内嵌 Netty 服务 | Server 通过 AKKA/HTTP/MU 直接调 Worker 上报地址 | Worker 主动建立 gRPC 双向流，Server 不回连业务 Pod |
| DB 锁不是调度集群共识 | `xxl_job_lock` + `FOR UPDATE` 全局锁 | `oms_lock` + `currentServer` + PING 选主 | openraft / lease shard / fencing token |
| 内存时间轮只能做加速，不能做事实源 | 60 秒 ringData，Admin 重启即丢 | InstanceTimeWheelService 承担延迟派发 | trigger_event 持久化，内存轮只做 near-time cache |
| 动态脚本/SQL/HTTP 参数必须有安全边界 | GLUE/Shell/Python/Node/PowerShell 宿主执行 | Groovy/SQL/HTTP/脚本下载执行攻击面大 | 多语言脚本运行时 + WASM/子进程/容器沙箱、URL policy、参数化 SQL、审计 |
| 工作流必须是一等状态机 | child_jobid 不是 DAG | DAG 存在但状态散落、Groovy 决策风险高 | workflow_event + typed context + safe expression |

因此，scheduler 的目标不是“复刻 PowerJob”，而是保留其有价值的功能模型（多调度方式、MapReduce、DAG、官方处理器），同时替换掉不适合云原生公共服务的通信、选举、状态、安全和可观测性设计。
关键源码依据（用于支撑后文设计取舍）：

- xxl-job 调度类型与 DB 锁：`xxl-job-admin/src/main/java/com/xxl/job/admin/scheduler/type/ScheduleTypeEnum.java`、`xxl-job-admin/src/main/resources/mapper/XxlJobLockMapper.xml`、`JobScheduleHelper.java`
- xxl-job Executor 反向调用与 child_jobid：`xxl-job-core/src/main/java/com/xxl/job/core/executor/XxlJobExecutor.java`、`xxl-job-admin/src/main/java/com/xxl/job/admin/scheduler/complete/JobCompleter.java`
- PowerJob 多协议与端口：`powerjob-server-starter/src/main/resources/application.properties`、`PowerTransportService.java`、`powerjob-remote-impl-akka/.../package-info.java`
- PowerJob Worker 入站地址与 DB 选举：`PowerJobWorker.java`、`ServerElectionService.java`、`DatabaseLockService.java`
- PowerJob 调度、H2、日志与 Groovy：`CoreScheduleTaskManager.java`、`PowerScheduleService.java`、`ConnectionFactory.java`、`OmsLogHandler.java`、`DecisionNodeHandler.java`

### 1.4 核心设计原则

1. **Simplicity over Flexibility** — 能用一种方式解决的，不用两种
2. **Protocol as Contract** — gRPC protobuf 即接口文档，即多语言 SDK
3. **Single Binary** — 编译产物为一个可执行文件，`./scheduler serve` 即启动
4. **Memory Safe by Default** — Rust 所有权模型消除整类内存 bug
5. **Cloud Native First** — K8s/Docker/容器部署是一等能力，Server 与 Worker 可部署在不同容器、namespace、集群、VPC 或云厂商中
6. **Zero Trust** — 默认 TLS + mTLS、RBAC、审计日志

---

## 2. 功能覆盖与竞品对照

> 设计目标：不是简单“100% 覆盖 PowerJob”，而是以 xxl-job 和 PowerJob 的源码事实为基线，保留 PowerJob 中有价值的功能模型，同时修正二者在通信、调度、工作流、安全、可观测性上的架构缺陷。

### 2.1 调度能力

| 功能 | xxl-job | PowerJob | scheduler | 增强重点 |
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

| 功能 | xxl-job | PowerJob | scheduler | 增强重点 |
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

| 功能 | xxl-job | PowerJob | scheduler | 安全/体验增强 |
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

| 功能 | xxl-job | PowerJob | scheduler | 增强重点 |
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
    subgraph Cluster["scheduler Cluster"]
        direction TB

        subgraph S1["Server #1 (Leader)"]
            SCH1["Scheduler"]
            WF1["Workflow Engine"]
            GW1["gRPC+HTTP Gateway"]
            UI1["Web UI (embedded)"]
        end

        subgraph S2["Server #2 (Follower)"]
            SCH2["Scheduler"]
            WF2["Workflow Engine"]
            GW2["gRPC+HTTP Gateway"]
            UI2["Web UI (embedded)"]
        end

        subgraph S3["Server #N (Follower)"]
            SCH3["Scheduler"]
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

### 4.1 scheduler Server

Server 是平台的核心，承担调度、工作流编排、集群管理、API 网关四大职责。

#### 4.1.1 模块架构

```mermaid
graph LR
    subgraph Server["scheduler-server"]
        MAIN["main.rs<br/>CLI + 启动"]
        CFG["config.rs<br/>TOML 配置"]
        SRV["server.rs<br/>服务组装"]

        subgraph Scheduler["scheduler/"]
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

| 特性 | PowerJob | scheduler |
|------|----------|----------|
| DAG 依赖 | ✅ | ✅ |
| 条件分支 | ❌ | ✅ (基于上下文的条件表达式) |
| 循环节点 | ❌ | ✅ (for/while 循环控制) |
| 子工作流 | ❌ | ✅ (工作流嵌套调用) |
| 节点超时 | 部分 | ✅ (全局 + 单节点超时) |
| 失败策略 | 重试 | ✅ 重试 / 跳过 / 暂停 / 回调 |
| 上下文传递 | KV HashMap | ✅ 强类型上下文 + JSON Schema 校验 |
| 任务排队 | ❌ (超限直接失败) | ✅ (可配置队列容量 + 优先级) |

### 4.2 scheduler Worker (SDK)

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
proto/scheduler/worker/v1/
├── worker.proto          # Worker 注册、心跳、任务接收
├── task.proto            # 任务定义、状态、结果
├── processor.proto       # 处理器协议
└── workflow.proto        # 工作流上下文
```

SDK 与示例统一目录（强约束）：

```text
sdks/
├── rust/
│   └── scheduler-worker-sdk/           # Rust Worker SDK crate (tonic)
├── java/
│   ├── scheduler-java/                    # 原生 Java SDK
│   ├── scheduler-spring/               # Spring 集成
│   └── scheduler-spring-boot-starter/          # Spring Boot 集成
├── go/
│   └── scheduler-go-sdk/               # 规划
├── python/
│   └── scheduler-python-sdk/           # 规划
└── nodejs/
    └── scheduler-nodejs-sdk/           # 规划

examples/
├── rust/
│   └── worker-demo/                    # Rust SDK demo worker / task processor
├── java/
│   └── spring-worker-demo/             # Java Spring Boot demo app，Gradle 构建，JDK 21+
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
- Rust SDK 已按规范迁移到 `sdks/rust/scheduler-worker-sdk`，Cargo workspace 已同步调整。
- 根 `Dockerfile` 只构建 scheduler 服务端镜像，不复制、不缓存、不构建 `sdks/` 与 `examples/`；SDK 与 Demo 必须作为独立构建产物验证。
- 独立发布约束：每个 SDK 必须可按语言生态独立发布；Rust SDK 不能依赖服务端 `crates/*` path dependency，必须内聚协议定义或依赖已发布协议包。
- Worker 注册约束：`worker_id` 必须由服务端生成并在 `WorkerRegistered` 下发；客户端只能上报可选 `client_instance_id` 作为实例提示，不能自行声明权威 ID。
- Worker 分发约束：`DispatchTask.processor_name` 是 SDK 侧处理器路由的显式字段；Job 定义与 Workflow job/map 节点均支持显式 `processor_name` 绑定，dispatcher 优先使用节点绑定，其次使用 Job 绑定，最后仅为历史数据回退到 `job_id`。
- Node 目录统一命名为 `nodejs`，避免和通用 node/graph 概念混淆。

**集成体验对比**：

```rust
// scheduler Rust SDK — 3 行代码集成
use scheduler_sdk::prelude::*;

#[scheduler::processor]
async fn my_task(ctx: TaskContext) -> TaskResult {
    let data = ctx.param("key")?;
    // 业务逻辑
    Ok(TaskResult::success("done"))
}

// main.rs
#[tokio::main]
async fn main() {
    scheduler::worker()
        .server("scheduler.example.com:9090")
        .app_name("my-service")
        .register(my_task)
        .start()
        .await?;
}
```

```python
# scheduler Python SDK
from scheduler import Worker, TaskContext, TaskResult

worker = Worker("scheduler.example.com:9090", app_name="my-service")

@worker.processor("my_task")
async def my_task(ctx: TaskContext) -> TaskResult:
    data = ctx.param("key")
    return TaskResult.success("done")

worker.start()
```

```go
// scheduler Go SDK
package main

import (
    scheduler "github.com/scheduler/sdk-go"
)

func main() {
    w := scheduler.NewWorker("scheduler.example.com:9090",
        scheduler.WithAppName("my-service"))

    w.Register("my_task", func(ctx scheduler.TaskContext) scheduler.TaskResult {
        return scheduler.Success("done")
    })

    w.Start()
}
```

对比 PowerJob 的集成方式——需要添加 Maven 依赖、配置 properties、实现 Java 接口、Spring Boot 启动——scheduler 的多语言 SDK 将集成成本降低到**任意语言 3-5 行代码**。

#### 4.2.3 Java Spring Boot Starter SDK

Java 端 SDK 优先支持 Spring Boot Starter 模式，目标是让现有 Spring Boot 业务以最小改造接入 scheduler Worker Tunnel。

**模块规划**：

```text
sdks/java/
├── settings.gradle.kts                  # Gradle multi-project settings
├── build.gradle.kts                     # Java 21+ toolchain、统一依赖版本与发布元数据
├── scheduler-java/                     # 原生 Java 集成：gRPC client、协议模型、通用 Worker runtime
├── scheduler-spring/                   # Spring 集成：@SchedulerProcessor 注册表与方法适配
└── scheduler-spring-boot-starter/              # Spring Boot 集成：AutoConfiguration、Properties、starter 聚合
```

Java SDK 三层 Gradle 模块约束：
- `scheduler-java`：原生 Java 集成，包含 Worker Tunnel gRPC client、协议生成、任务上下文与结果模型。
- `scheduler-spring`：Spring Framework 集成，包含 `@SchedulerProcessor` 扫描、注册表和方法适配，不包含 Spring Boot autoconfigure。
- `scheduler-spring-boot-starter`：Spring Boot 集成，包含 Properties、AutoConfiguration 和 starter 聚合能力，依赖 `scheduler-spring`。

Java SDK 构建约束：
- 必须使用 Gradle（优先 Kotlin DSL：`settings.gradle.kts` / `build.gradle.kts`），不再使用 Maven `pom.xml` 作为主构建。
- Java toolchain 与源码/目标兼容级别必须支持 JDK 21+。
- Spring Boot Starter 模式继续保留，业务侧只需依赖 starter。
- 当前 Java Core SDK 已提供真实 gRPC Worker Tunnel 客户端：注册时只发送 `client_instance_id`，读取服务端下发的权威 `worker_id`，并用于心跳、任务日志和任务结果上报；Spring Boot demo 默认 dry-run，可通过配置切换到 live tunnel。
- CI / 本地验证命令统一为 `./sdks/java/gradlew -p sdks/java test`；每个 Java SDK 子模块也必须支持 Gradle 单模块任务（如 `./sdks/java/gradlew -p sdks/java :scheduler-java:test`）；Maven 骨架与 `mvn -f sdks/java/pom.xml test` 文档引用不得再新增。

**业务侧使用方式**：

```java
@Component
public class BillingTasks {
    @SchedulerProcessor("billing.reconcile")
    public TaskResult reconcile(TaskContext context) {
        return TaskResult.success("ok");
    }
}
```

```yaml
scheduler:
  server: https://scheduler.example.com
  app-name: billing-service
  worker-pool: prod-cn
  namespace: finance
  labels:
    region: cn
    runtime: spring-boot
```

Starter 需要提供：

- `@EnableSchedulerWorker` 或自动启用的 Spring Boot auto-configuration。
- `@SchedulerProcessor` 注解扫描和方法适配。
- 与 Server 的 Worker Tunnel 主动连接、注册、心跳、状态上报、日志上报和日志订阅。当前已完成真实 gRPC 连接、注册、心跳、日志、任务结果回传，并已支持将 `@SchedulerProcessor` 方法适配为真实任务处理器（通过 `DispatchTask.processor_name` 匹配 processor name，payload 支持 UTF-8 String / byte[] / TaskContext；空值兼容回退到 `job_id`）。
- Spring Boot lifecycle 集成：应用启动后连接，`ContextClosedEvent` 时 drain/优雅下线。
- Micrometer 指标、Actuator health indicator、结构化日志上下文。
- mTLS / token / cert rotation 配置入口。
- 默认不暴露入站端口，不要求业务 Service 被 scheduler 访问。

#### 4.2.4 动态脚本处理器设计

动态脚本是一等处理器能力，用于低频运维任务、轻量数据处理、迁移脚本、Webhook 编排和临时自动化。设计目标是**多语言可用，但默认不信任脚本**：脚本永远不在 scheduler Server 进程内执行，只能在 Worker 侧的受控执行环境中运行。

**支持语言分层**：

| 级别 | 语言/运行时 | 适用场景 | 安全策略 |
|------|-------------|----------|----------|
| 默认支持 | Shell、Python、Node.js/TypeScript、PowerShell | 运维脚本、数据处理、API 编排 | 子进程沙箱，默认无网络/只读文件系统/资源限额 |
| 安全表达式 | Rhai / CEL / JSONLogic | 工作流条件、参数转换、轻量计算 | 嵌入式解释器，禁用反射、IO、网络、进程启动 |
| 高安全插件 | WASM/WASI | 可复用处理器、跨语言插件、强隔离任务 | Wasmtime fuel/epoch、capability-based WASI、签名校验 |
| 企业扩展 | 容器化脚本运行器 | 需要系统依赖或复杂运行时的脚本 | 独立 Pod/容器，seccomp/AppArmor、NetworkPolicy、只读 rootfs |

**执行安全边界**：

1. **Server 不执行用户代码**：Server 只保存脚本定义、版本、审批状态和策略，实际执行由匹配 Worker Pool 完成。
2. **脚本版本化与签名**：脚本内容按 content hash 存储；每次更新自动产生新版本记录（content、policy 变更均产生版本）；支持任意两个版本间的 diff 对比（content diff、policy diff）；生产环境脚本必须经过审批、签名或可信发布流水线。
3. **最小权限 capability**：脚本声明所需能力，例如 `network.egress`、`fs.read:/data/input`、`secret:db-readonly`；未声明能力默认不可用。
4. **资源限制**：每次执行强制 timeout、CPU quota、内存上限、输出大小、日志速率、最大并发和重试预算。
5. **文件系统隔离**：默认临时工作目录；只读挂载输入；输出通过受控 artifact API 写入；禁止访问宿主敏感路径。
6. **网络隔离**：默认禁止出站网络；允许时必须经过 URL policy、DNS pinning、内网/metadata 地址阻断、TLS 校验和请求审计。
7. **凭证隔离**：脚本只能通过 Secret reference 获取临时凭证；日志和错误栈自动脱敏；禁止把密钥作为普通参数明文存储。
8. **危险能力审批**：启用网络、写文件、执行外部命令、访问 Secret、长超时、高资源配额等能力需要策略审批。
9. **审计与可追溯**：记录脚本版本、提交人、审批人、执行 Worker、输入摘要、能力清单、资源用量、网络目标和 artifact hash；所有脚本更新必须保留历史版本并支持 diff 对比。

**推荐执行路径**：

```text
Job Definition
  -> Script Processor(language, code_ref, runtime_policy)
  -> Scheduler 选择具备对应 runtime 的 Worker Pool
  -> Worker 拉取脚本版本并校验签名/hash
  -> Sandbox Runner 创建隔离环境
  -> 执行脚本并流式上报日志/指标/artifact
  -> 清理临时目录并提交审计事件
```

**禁止项**：

- 禁止在 Server 进程中嵌入 Groovy/Python/Node 等解释器执行用户脚本。
- 禁止默认继承 Worker 进程环境变量、宿主网络和宿主文件系统。
- 禁止脚本直接读取平台数据库或内部管理 API；必须通过受控 Service Account 与 RBAC 授权。
- 禁止把“动态脚本”作为绕过正式 SDK、权限和审计的后门。



### 4.3 scheduler CLI

```bash
# 服务管理
scheduler serve                           # 启动 server（单机模式，SQLite）
scheduler serve --cluster --db postgres   # 启动 server（集群模式）

# 任务管理
scheduler job create --file job.yaml      # 从 YAML 创建任务
scheduler job list --app my-service       # 列出任务
scheduler job trigger <job-id>            # 手动触发
scheduler job cancel <instance-id>        # 取消执行

# 工作流管理
scheduler workflow create --file flow.yaml
scheduler workflow trigger <wf-id>
scheduler workflow visualize <wf-id>      # 终端 ASCII 可视化

# 集群管理
scheduler cluster status                  # 集群健康状态
scheduler cluster workers                 # Worker 列表
scheduler cluster reschedule --app xxx    # 强制重新调度

# 数据管理
scheduler migrate                         # 执行数据库迁移
scheduler export --format json            # 导出任务定义
scheduler import --file backup.json       # 导入任务定义
```

---

## 5. 通信协议设计

### 5.1 协议选型：gRPC (HTTP/2)

xxl-job 与 PowerJob 的通信问题不是“协议实现细节”，而是公共服务化能力的根本边界：

- xxl-job：Admin 通过 HTTP 反向调用 Executor 内嵌 Netty 服务，Executor 必须注册可访问地址。
- PowerJob：Server 通过 AKKA/HTTP/MU 反向调用 Worker，上报地址还要区分 bind address 与 external address。
- scheduler：Worker 主动建立 gRPC/HTTP2 长连接隧道，Server 不要求直连 Worker Pod，也不依赖 Worker 可被公网、跨 namespace 或跨集群访问。

| 场景 | xxl-job | PowerJob | scheduler |
|------|---------|----------|-----------|
| Server/Admin → Worker/Executor 分发 | HTTP 调 Executor `/run` | Akka TCP / HTTP / MU 调 Worker | gRPC 双向流下发 `DispatchTask` |
| Worker/Executor → Server 心跳 | Executor 注册/心跳到 Admin | Akka/HTTP/MU 心跳上报 | gRPC stream heartbeat，连接即租约 |
| 日志传输 | Admin 轮询 Executor 本地日志 | Worker 批量上报，队列满或 Server 不可用可能丢 | gRPC Client Stream，背压 + 可选 WAL |
| 状态上报 | HTTP callback | Akka/HTTP/MU 上报 | gRPC stream/unary，attempt token 幂等 |
| Server 集群协调 | DB `FOR UPDATE` 全局锁 | DB lock + currentServer + PING | Raft / lease shard / fencing token |
| K8s/Docker/NAT/多级网关适配 | 需要 Executor 入站可达 | 需要 Worker 入站可达，多协议多端口 | 只需要 Worker 出站访问 scheduler tunnel endpoint，支持跨 namespace/cluster/VPC |
| 管理 API | HTTP MVC | HTTP REST | REST + gRPC Gateway + gRPC reflection |

因此 scheduler 使用**单一 gRPC 协议**解决 Worker 通信、任务分发、日志流、状态上报和集群内部 RPC；REST 只作为管理面 API，不进入核心执行链路。

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

PowerJob 需要 4 个端口 (7700, 10086, 10010, 10077)。scheduler 只需**1 个端口**：

```
scheduler-server:9090
├── gRPC (h2)           — Worker Tunnel + API + 集群 RPC
├── HTTP/1.1 (REST)     — Web 控制台 + REST API
└── WebSocket           — 浏览器实时日志（可选，gRPC-Web 亦可）
```

### 5.4 Worker 主动连接模型

scheduler 的 Worker Tunnel 是对 xxl-job / PowerJob 反向调用模型的直接修正，也是跨容器、跨 namespace、跨集群部署的核心能力。Worker 注册、心跳、任务分发、取消、日志、证书轮换和配置下发都复用同一条由 Worker 主动发起的长连接。

```protobuf
service WorkerTunnelService {
  rpc Connect(stream WorkerMessage) returns (stream ServerMessage);
}
```

消息类型：

| 方向 | 消息 |
|------|------|
| Worker → Server | Register、Heartbeat、TaskStatus、TaskResult、LogChunk、Metrics、LeaseRenew |
| Server → Worker | DispatchTask、CancelTask、Drain、UpdateConfig、RotateCert、Ping |

该模型带来的直接收益：

1. **无业务入站端口**：业务 Pod/容器不需要为调度暴露 Service、Ingress、NodePort 或公网端口。
2. **穿透多级网络层级**：只要求 Worker 能出站访问 scheduler tunnel endpoint；中间可以是 Docker bridge、K8s Service、Ingress、API Gateway、Service Mesh、NAT Gateway、VPN、专线或跨云负载均衡。
3. **反向调用走既有通道**：Server 对 Worker 的 DispatchTask、CancelTask、Drain、RotateCert 等“反向调用”不是新建到 Worker 的连接，而是写回 Worker 已建立的双向流。
4. **跨 namespace/cluster/VPC 简化**：Worker 所在网络无需被 scheduler 路由可达；注册时上报 app、namespace、cluster、region、tenant、capabilities 和 labels，Server 按逻辑属性寻址。
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
- 认证：支持 Session Cookie、Bearer Token、OIDC JWT、Service Account Token。
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
| Scripts | `GET/POST /api/v1/scripts`、`POST /api/v1/scripts/{script}:publish`、`:approve`、`:rollback`、`GET /api/v1/scripts/{script}/versions`、`GET /api/v1/scripts/{script}/diff?v1=&v2=` | 动态脚本版本、发布、审批、回滚、版本历史与 diff 对比 |
| Secrets | `GET/POST /api/v1/secrets`、`POST /api/v1/secrets/{secret}:rotate` | Secret reference 管理 |
| Alerts | `GET/POST /api/v1/alert-rules`、`GET/POST /api/v1/notification-channels` | 告警规则与通知渠道 |
| Audit | `GET /api/v1/audit-logs` | 审计查询与导出 |
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
    participant CRON as Scheduler<br/>(CRON Engine)
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
    participant SCH as Scheduler
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
    participant SCH as Scheduler
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
| **SQLite** | 单机/开发/嵌入式 | ✅ 必须支持 | 零配置，单文件部署，`./scheduler serve` 直接可用 |
| **MySQL 8.x** | 中小规模生产 | ✅ 必须支持 | PowerJob 用户迁移首选 |
| **PostgreSQL 15+** | 大规模生产/集群 | ✅ 必须支持 | 高并发、JSONB、Citus 水平扩展 |
| **CockroachDB** | 地理分布/云原生 | ✅ 必须支持 | 分布式 SQL，Serverless 友好 |
> Phase 2 implementation note: `scheduler-storage` now enables `sqlx-postgres` alongside SQLite/MySQL. PostgreSQL and CockroachDB use `postgres://` URLs; CockroachDB relies on PostgreSQL wire protocol compatibility. Database relationships remain soft-linked by id fields only; no foreign keys are introduced for any backend.

| **MariaDB** | MySQL 兼容替代 | 🔄 兼容支持 | 通过 MySQL driver 兼容 |

### 7.3 存储抽象层设计

```mermaid
flowchart TD
    subgraph App["业务层"]
        SCH["Scheduler"]
        WF["Workflow Engine"]
        AUTH["Auth Module"]
    end

    subgraph Repo["Repository 抽象层<br/>(scheduler 自定义)"]
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
    id              BIGINT PRIMARY KEY,
    user_id         BIGINT,
    action          VARCHAR(64) NOT NULL,
    resource_type   VARCHAR(32) NOT NULL,
    resource_id     BIGINT NOT NULL,
    detail          JSONB,
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

scheduler 的部署目标是**容器优先、网络边界无关**。Server 和 Worker 必须可以部署在不同容器、不同 Docker 网络、不同 K8s namespace、不同 K8s 集群、不同 VPC/机房甚至不同云厂商中。平台不得要求 Worker 暴露入站端口；所有注册、心跳和反向调度指令都必须通过 Worker 主动建立的 tunnel 完成。

部署硬性约束：

1. **K8s 必须一等支持**：提供 Helm Chart、Kustomize 示例、Gateway API/Ingress 示例、ServiceMonitor、NetworkPolicy、PodDisruptionBudget 和多副本 StatefulSet/Deployment 模板。
2. **Docker 必须一等支持**：提供 server 镜像、worker 镜像、docker compose 示例、本地开发网络示例和 scratch/distroless 生产镜像。
3. **Server/Worker 可独立部署**：Server 可在平台集群，Worker 可在业务集群；Worker 可作为 sidecar、独立 Deployment、DaemonSet、Job Runner 或嵌入 SDK 运行。
4. **跨网络反向调用**：Server 不需要也不允许直接拨 Worker 地址；反向指令必须复用 Worker→Server 的长连接。
5. **单入口最小暴露**：默认只暴露 scheduler server 的 443/9090 tunnel/API 入口；业务命名空间默认不创建 Worker 入站 Service。

### 8.1 单机模式 (Standalone)

**零配置，开箱即用**——这是 scheduler 与 PowerJob 最大的用户体验差异。

```bash
# 下载单文件 (约 15MB，含前端)
curl -LO https://github.com/scheduler/scheduler/releases/latest/download/scheduler-linux-amd64
chmod +x scheduler-linux-amd64

# 启动 (自动创建 SQLite 数据库)
./scheduler-linux-amd64 serve

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

scheduler:
  1. 下载 scheduler (~15MB)
  2. ./scheduler serve
  3. 浏览器自动打开 http://localhost:9090
  (启动时间 < 1s)
```

### 8.2 Docker / Compose 部署

Server 与 Worker 可以在同一个 Docker 网络内，也可以位于不同 Docker host。Worker 只需要配置 `SCHEDULER_SERVER` 指向可出站访问的 server tunnel endpoint。

```bash
# 单机 (约 20MB 镜像，基于 scratch)
docker run -d \
  --name scheduler \
  -p 9090:9090 \
  -v scheduler-data:/var/lib/scheduler \
  ghcr.io/scheduler/scheduler:latest

# 使用 MySQL
docker run -d \
  --name scheduler \
  -p 9090:9090 \
  -e SCHEDULER_DB_URL="mysql://user:pass@mysql:3306/scheduler" \
  ghcr.io/scheduler/scheduler:latest

# 使用 PostgreSQL
docker run -d \
  --name scheduler \
  -p 9090:9090 \
  -e SCHEDULER_DB_URL="postgres://user:pass@pg:5432/scheduler" \
  ghcr.io/scheduler/scheduler:latest

# Worker 独立容器：不暴露端口，只主动连 server
docker run -d \
  --name scheduler-worker \
  -e SCHEDULER_SERVER="https://scheduler.example.com" \
  -e SCHEDULER_APP_NAME="billing-worker" \
  -e SCHEDULER_WORKER_POOL="prod-cn" \
  ghcr.io/scheduler/scheduler-worker:latest
```

`docker compose` 推荐提供 `scheduler-server`、`scheduler-worker`、`postgres`、`prometheus` 四类服务模板；Worker 服务不声明 `ports`，只声明出站网络。

### 8.3 Kubernetes 集群部署架构

```mermaid
graph TB
    subgraph K8s["Kubernetes Cluster"]
        direction TB

        LB["Ingress / LoadBalancer<br/>:443 TLS"]

        subgraph NS["Namespace: scheduler"]
            direction TB

            subgraph StatefulSet["scheduler-server (StatefulSet, 3 replicas)"]
                S1["Pod: Server 1<br/>port 9090<br/>Leader"]
                S2["Pod: Server 2<br/>port 9090<br/>Follower"]
                S3["Pod: Server 3<br/>port 9090<br/>Follower"]
            end

            SVC["Service: scheduler<br/>ClusterIP :9090"]

            subgraph CM["ConfigMap / Secrets"]
                CFG["scheduler-config<br/>TOML 配置"]
                SEC["scheduler-secrets<br/>DB 密码 / TLS 证书"]
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
                SC1["scheduler-worker<br/>sidecar"]
            end
            subgraph Pod2["Pod: app-b"]
                C2["app container<br/>:8080"]
                SC2["scheduler-worker<br/>sidecar"]
            end
        end
    end

    subgraph RemoteK8s["Remote Kubernetes / Docker / VM"]
        RW["remote scheduler-worker<br/>no inbound port"]
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

scheduler Worker 支持以 Sidecar 模式运行在 K8s Pod 中，无需修改业务应用代码：

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
      - name: scheduler-worker
        image: ghcr.io/scheduler/scheduler-worker:latest
        env:
        - name: SCHEDULER_SERVER
          value: "https://scheduler.scheduler.svc.cluster.local:9090"
        - name: SCHEDULER_APP_NAME
          value: "my-business-app"
        - name: SCHEDULER_PROCESSORS
          value: "http://localhost:8080/tasks/*"
```

### 8.5 Worker 部署形态

| 形态 | 适用场景 | 网络要求 | 备注 |
|------|----------|----------|------|
| SDK 嵌入业务进程 | 业务代码直接实现处理器 | 业务进程出站访问 scheduler | 延迟最低，适合核心业务任务；Java/Rust SDK 已按服务端下发 worker_id 模型接入 Worker Tunnel |
| Sidecar | 不希望业务进程直接管理调度连接 | Pod 内 localhost 调业务容器；sidecar 出站访问 scheduler | 默认 K8s 推荐模式 |
| 独立 Worker Deployment | HTTP/gRPC/SQL/Script 等通用任务 | Worker 出站访问 scheduler 和目标系统 | 适合共享 worker pool |
| DaemonSet | 节点级任务、文件清理、宿主观测 | 每节点 Worker 出站访问 scheduler | 需更严格权限策略 |
| 跨集群 Worker Gateway | 远端集群集中接入 | gateway 出站访问中心 scheduler | 可减少远端集群出口连接数 |
| 容器化脚本 Runner | 动态脚本/依赖复杂任务 | runner 出站访问 scheduler | 配合 seccomp/AppArmor/NetworkPolicy |

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

        subgraph scheduler["scheduler 集群"]
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
    MON -->|Prometheus| scheduler

    ING --> scheduler

    S1 --- PG_M
    S2 --- PG_M
    S3 --- PG_M
    PG_M --> PG_R1

    WA -->|gRPC| scheduler
    WB -->|gRPC| scheduler
    WC -->|gRPC| scheduler

    scheduler --> OBJ
    scheduler --> NFS
```

### 8.7 K8s / Docker 公共服务化部署要点

本设计明确把“公共调度服务”作为部署目标，而不是只服务单个业务系统。与 xxl-job / PowerJob 相比，scheduler 在 K8s、Docker 和跨集群容器部署中的关键差异如下：

| 场景 | xxl-job / PowerJob 的问题 | scheduler 方案 |
|------|---------------------------|----------------|
| 同集群多 namespace | 每个 Executor/Worker 都要暴露可回连地址，NetworkPolicy 和 Service 管理复杂 | Worker 只出站连接 scheduler Service，不要求业务 namespace 暴露调度端口 |
| 多集群/多 VPC | 中心调度服务要能路由到远端 Pod/Service | 远端 Worker 主动连接中心或区域 gateway；Server 通过既有 tunnel 反向下发任务，天然跨 NAT/防火墙/网关 |
| 业务 Pod 重启/扩缩容 | Executor/Worker 地址变化导致注册表与实际可达性不一致 | 连接断开即下线，重新连接即注册，租约过期自动重调度 |
| Service Mesh | 多协议、多端口、多方向流量策略复杂 | 单 gRPC h2 出站连接，mTLS/限流/审计集中治理 |
| 本地 Docker/Compose 调试 | external address/port 很容易配置错误 | 只需配置 `SCHEDULER_SERVER`，Worker 容器不声明入站 ports，不需要暴露给 Server |

因此 Helm Chart 和 Compose 模板默认不为业务 Worker 创建入站 Service/ports；只有 scheduler server 暴露管理面和 Worker Tunnel 入口。

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

| 安全域 | xxl-job 问题 | PowerJob 问题 | scheduler 解决方案 |
|--------|--------------|---------------|--------------------|
| 远程代码执行 | GLUE_GROOVY、Shell/Python/Node/PowerShell 等在宿主执行 | GroovyEvaluator 决策节点、动态脚本、外部 JAR 容器扩大攻击面 | 多语言脚本沙箱 + WASM/容器隔离；Server 不执行用户代码；脚本签名、审批、能力声明与最小权限 |
| SQL 注入/危险 SQL | 核心平台较少内置 SQL 任务，但缺统一 SQL 治理 | `detailPlus` customQuery 黑名单过滤后拼接；SQL Processor 默认依赖用户注册 validator | 参数化 SQL 模板、数据源白名单、dry-run、审批、审计 |
| SSRF/内网探测 | HTTP 类任务缺平台级 egress policy | HTTP Processor 允许任务参数指定 URL，默认缺内网地址治理 | URL 白名单/黑名单、DNS pinning、禁止 metadata/link-local/内网网段 |
| 传输认证 | 默认 `default_token`，可为空 | Worker/Server 多协议通信缺统一 mTLS 默认模型 | TLS/mTLS、Worker cert rotation、bootstrap token 最小权限 |
| 权限模型 | 用户/执行器维度较粗 | V5.x 权限增强但 OpenAPI/控制台/Worker 链路仍不统一 | gRPC/REST 方法级鉴权，namespace/app/worker pool scope |
| 凭证治理 | 配置文件明文较常见 | 配置与任务参数容易携带明文凭证 | Secret reference、Vault/KMS/K8s Secret、日志脱敏 |
| 审计 | 操作审计不足 | 审计不完整 | 全操作审计 + SIEM/OTLP 导出 |
| 网络暴露 | Executor 必须暴露入站端口 | Worker 必须暴露入站端口，多端口多协议 | Worker 仅出站连接 scheduler，减少攻击面 |

---

## 11. 性能分析

### 11.1 预期性能指标

| 指标 | PowerJob (Java/Akka) | scheduler (Rust/gRPC) | 预期提升 |
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
        SCH["Scheduler<br/>调度事件"]
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
# 调度指标
scheduler_scheduler_tasks_dispatched_total{app, job_type}
scheduler_scheduler_tasks_succeeded_total{app, job_type}
scheduler_scheduler_tasks_failed_total{app, job_type}
scheduler_scheduler_dispatch_duration_seconds{app}        # histogram

# 队列指标
scheduler_scheduler_queue_length{app}
scheduler_scheduler_queue_capacity{app}

# Worker 指标
scheduler_worker_active_count{app}
scheduler_worker_heartbeat_latency_seconds{app, worker}
scheduler_worker_task_duration_seconds{app, processor}    # histogram

# 系统指标
scheduler_server_uptime_seconds
scheduler_server_connections_active
scheduler_db_query_duration_seconds{operation}            # histogram
scheduler_grpc_request_duration_seconds{method}           # histogram
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
| 共识算法 | openraft | 0.9+ | Server 集群 Raft 共识 |
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
scheduler/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── clippy.toml
├── deny.toml
│
├── proto/
│   ├── scheduler/
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
│   ├── scheduler-server/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── config.rs
│   │   │   ├── server.rs
│   │   │   ├── scheduler/
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
│   ├── scheduler-sdk/
│   ├── scheduler-client/
│   ├── scheduler-proto/
│   ├── scheduler-common/
│   └── scheduler-wasm/
│
├── sdks/                             # 多语言 SDK
│   ├── rust/scheduler-worker-sdk/    # Rust Worker SDK crate
│   ├── java/scheduler-java/           # 原生 Java SDK
│   ├── java/scheduler-spring/         # Spring 集成
│   ├── java/scheduler-spring-boot-starter/    # Spring Boot 集成
│   ├── go/scheduler-go-sdk/           # 规划
│   ├── python/scheduler-python-sdk/   # 规划
│   └── nodejs/scheduler-nodejs-sdk/   # 规划
│
├── examples/                         # SDK demo 项目，按 sdks/ 语言结构对齐
│   ├── rust/worker-demo/
│   ├── java/spring-worker-demo/
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

- [x] 项目脚手架 (workspace, CI, root binary entrypoint)
- [x] gRPC 协议定义与代码生成（Worker Tunnel proto + server streaming skeleton）
- [x] SeaORM 存储层 + SQLite + MySQL 迁移（SQLite dev DB 已验证，MySQL migration 通过 SeaORM feature 启用）
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
- [x] Docker 镜像构建（server 多阶段镜像 + Web nginx 镜像 + Compose/K8s 基础部署）
- [x] CLI 基础命令（`serve --config`）

### Phase 2: 工作流与分布式 (月 4-6)

**目标**：覆盖 PowerJob 的全部调度模式。

- [x] DAG 工作流引擎基础（定义存储、DAG 校验、最小 run API；可视化编排后续增强）
- [x] Map / MapReduce 执行模式（workflow_shards + materialize + shard job_instance/dispatch_queue 软关联）
- [x] 子工作流嵌套（节点引用 child_workflow_id + 子实例软关联 + 子实例终态回写父节点）
- [x] PostgreSQL + CockroachDB 存储支持（SeaORM/sqlx-postgres feature + `postgres://` 配置模板；CockroachDB 复用 PostgreSQL wire protocol）
- [ ] Server 集群 (Raft 共识)
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
- [x] Worker TaskResult 自动推进 Workflow（按 job_instance_id 软关联回写 workflow_node_instance / workflow_shard，并按边条件入队后继节点）
- [x] Workflow shard 完成回调与聚合推进（`POST /api/v1/workflow-shards/{id}/complete` 写入 output/status，全部成功后自动推进后继，失败时走失败边）
- [x] Workflow 操作审计日志（create/update/validate/dry-run/run/advance/materialize/recover 管理与执行动作写入 audit_logs）
- [x] Dispatch queue 最小租约与 claim API（lease_owner / lease_until + SQLite 兼容迁移；`POST /api/v1/dispatch-queue:claim` 支持按租约占用队列项）
- [x] Dispatch queue 原子 claim 与 dispatcher 接入（DB 条件更新抢占租约、过期 pending lease 回收、workflow queued node 和 single job dispatch 统一走 dispatch_queue）
- [x] SSE 实时实例事件骨架（instance_events + /events/instances/:id/stream；WebSocket 后续）

### Phase 3: 企业级特性 (月 7-9)

**目标**：可安全地在生产环境大规模部署。

> 020 review remediation 结论：015-019 中已完成项若仅为骨架，路线图必须明确标注“骨架/基础”，不得把未接入真实执行链路、规则引擎或治理闭环的能力标为完全完成。

- [x] RBAC 权限系统（021 已完成最小 permission/resource/action；OIDC/API Token/多租户 scope 后续继续增强）
- [ ] OIDC/SSO 集成
- [ ] mTLS 传输加密
- [x] Web 前端路由与导航治理基础（React Router v7、路由守卫、URL 持久化、菜单与路由对齐）
  - [ ] 路由 meta、懒加载、统一 403/401 与 URL 查询参数治理
- [x] 审计日志骨架（`audit_logs` 表、Repository、HTTP API、关键写操作埋点）
  - [ ] 审计 before/after、trace_id、失败结果、分页过滤与导出治理
- [ ] Web UI 危险操作二次确认、权限感知操作
  - [x] Web UI 审计日志查询页面（按操作类型筛选）
- [ ] WASM 沙箱处理器
- [ ] 多语言动态脚本处理器（Python/Node/Shell/PowerShell/Rhai）
  - [x] 脚本定义 Storage / Migration / Repository / HTTP CRUD API / OpenAPI
  - [x] Web 脚本管理页面（列表、创建、审批、启用/禁用、删除）
  - [x] 脚本版本历史表（`script_versions`），创建和更新时产生不可变版本快照
  - [x] 版本 diff 对比 API（`GET /api/v1/scripts/{id}/diff?v1=&v2=`）与 Web 侧 diff 视图
  - [ ] 发布指针、回滚 API、审批流状态机与 Worker 侧执行版本绑定
  - [x] 脚本编辑器语法高亮（CodeMirror 6 Shell/Python/Node）
  - [ ] Worker 侧沙箱执行器（子进程/容器/WASM）
- [ ] 脚本策略引擎（能力声明、审批、资源限制、网络/文件策略）
- [ ] 告警系统 (邮件/Slack/钉钉/飞书/企业微信/PagerDuty)
  - [x] AlertRule / AlertCondition / AlertDispatcher 安全 Webhook 通知骨架
  - [ ] 告警规则 API、事件接入、去重静默、通知历史、恢复通知
- [ ] Prometheus 指标 + Grafana Dashboard 模板
  - [x] Prometheus 指标端点（`/metrics`）与 HTTP/Worker 最小指标
  - [ ] Grafana Dashboard、调度延迟、实例状态与业务 SLO 指标
- [ ] OpenTelemetry 分布式追踪
- [ ] Java Spring Boot Starter SDK（优先）
  - [x] Gradle 多模块骨架：java-core / spring-boot-autoconfigure / spring-boot-starter（JDK 21+；已替换 Maven 骨架）
  - [x] `@SchedulerProcessor` 注解扫描与 auto-configuration 骨架
  - [x] Java gRPC Worker Tunnel 真实连接与心跳
- [ ] Node.js SDK
- [x] Java Core SDK
- [x] Worker processor binding model（Job 定义与 Workflow job/map 节点支持 `processor_name`，Worker dispatch 按 processor name 路由，legacy 数据回退 `job_id`）
- [x] SDK 目录规范迁移：Rust SDK -> `sdks/rust/scheduler-worker-sdk`，Java SDK -> Gradle/JDK21+，新增 `examples/<language>/<demo-name>` demo 骨架，并补齐 Rust / Java 可独立运行 demo 基础
- [ ] K8s Helm Chart
- [ ] PowerJob 迁移工具

### Phase 4: 高级能力 (月 10-12)

**目标**：超越 PowerJob，建立差异化竞争力。

- [ ] Go SDK + Python SDK（从 Phase 2 后置；待核心分布式/日志能力稳定后实现）
- [ ] 任务依赖自动发现与拓扑可视化
- [ ] 智能调度 (基于历史数据的资源预测)
- [ ] 多租户隔离增强
- [ ] 插件系统 (自定义处理器类型、告警通道)
- [ ] Terraform Provider
- [ ] Webhook 入站/出站
- [ ] 任务版本管理与回滚
- [ ] 灰度发布支持 (任务 A/B 测试)

### 15.5 创新能力清单

这些能力不单独作为“附录”存在，而是贯穿调度、执行、工作流、安全和运维设计：

| 能力 | 说明 | 所属阶段 |
|------|------|----------|
| Worker 主动连接公共服务 | 无业务入站端口，Server/Worker 可分离部署到不同容器、namespace、集群、VPC；反向调用复用 Worker tunnel 穿透多级网络 | Phase 1-2 |
| GitOps/IaC | YAML、K8s CRD、Terraform Provider、PR diff、变更审计 | Phase 4 |
| 任务版本与灰度 | Job version、canary、按 worker tag 灰度、失败自动回滚 | Phase 4 |
| 调度仿真 | 变更前模拟未来 N 次触发、misfire 结果、资源占用 | Phase 4 |
| 平台管理控制台 | 嵌入式 Web UI + HTTP/OpenAPI 管理接口，覆盖任务、实例、工作流、Worker、脚本、安全、审计和告警 | Phase 1-4 |
| 工作流回放 | 基于 workflow_event 重放实例，支持事故复盘 bundle | Phase 4 |
| 智能调度 | 基于历史耗时、Worker 负载、失败率进行资源预测和调度推荐 | Phase 4 |
| 策略引擎 | OPA/Rego 或内置 DSL，控制 Shell/SQL/HTTP/生产变更审批 | Phase 3-4 |
| WASM 插件 | 语言无关、安全沙箱、插件签名与版本管理 | Phase 3-4 |
| 多语言动态脚本 | Python/Node/Shell/PowerShell/Rhai 等受控运行，统一沙箱、能力声明、资源限制、审批和审计 | Phase 3-4 |
| 事件驱动 | Webhook、Kafka/NATS/Redis Stream 触发源，出站 HMAC 回调 | Phase 4 |
| 多租户配额 | namespace/app/worker pool 级并发、QPS、日志量、存储 TTL | Phase 3 |

---

## 16. xxl-job / PowerJob 迁移指南 (概要)

为了降低既有用户迁移成本，scheduler 将同时提供 xxl-job 与 PowerJob 的迁移工具，但迁移策略不同：xxl-job 偏“补能力”，PowerJob 偏“替换架构债”。

### 16.1 从 xxl-job 迁移

可自动迁移：

1. `xxl_job_group` → scheduler app / worker pool。
2. `xxl_job_info` → scheduler job definition。
3. `CRON` / `FIX_RATE` → scheduler schedule。
4. `executor_handler` → processor name。
5. `executor_route_strategy` → worker selector / dispatch policy。
6. `child_jobid` → 简单 DAG workflow 边。

需要人工确认：

- GLUE/Shell/Python/Node/PowerShell 等动态脚本任务迁移到 Script Processor，并补充语言运行时、沙箱策略、资源限制、网络/文件策略和审批策略。
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

- Groovy decision 脚本 → scheduler 安全表达式或 WASM 处理器。
- SQL Processor → 数据源白名单、参数化模板、dry-run 和审批策略。
- HTTP Processor → URL policy、内网地址阻断和签名配置。
- Dynamic JAR / External Processor → SDK Processor、WASM 或容器任务。
- Worker tag / protocol / external address → Worker Pool selector。

### 16.3 双跑与回滚

迁移工具应支持：

- `scheduler migrate --from xxl-job --db mysql://... --dry-run`
- `scheduler migrate --from powerjob --db mysql://... --dry-run`
- 生成迁移报告：不可迁移项、风险项、安全策略缺口、下次触发时间差异。
- 支持 xxl-job / PowerJob 与 scheduler 双跑一段时间，通过实例结果和日志对账后再切流。

---

## 17. 风险与应对

| 风险 | 影响 | 应对策略 |
|------|------|----------|
| Rust 开发者招聘难 | 开发速度 | 核心用 Rust，SDK 层可用各语言原生开发；文档驱动社区贡献 |
| WASM 生态不够成熟 | 处理器灵活性 | WASM 作为高安全插件方案；动态脚本优先走成熟语言运行时 + 子进程/容器沙箱，二者共享策略与审计模型 |
| Raft 实现复杂度 | 集群稳定性 | 使用成熟开源实现 (openraft)，避免自研 |
| 前端开发资源 | UI 体验 | 前端固定在 `./web`，使用 React + Ant Design + Bun，保持独立工程边界 |
| 与 PowerJob 功能差距 | 用户迁移意愿 | 严格对照表 + 迁移工具 + 兼容 API |

---

## 18. 总结

scheduler 的核心判断是：**xxl-job 的问题是能力不够，PowerJob 的问题是功能堆叠后架构债过重**。二者都能解决一部分“定时任务”问题，但都不适合作为企业平台中面向多团队、多语言、多集群、多租户的公共任务调度基础设施。

| 维度 | xxl-job 的思路 | PowerJob 的思路 | scheduler 的思路 |
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

**结论**：scheduler 不是对 PowerJob 或 xxl-job 的简单替代实现，而是面向企业平台的重新设计。最核心的价值不是“多几个调度类型”，而是把任务调度的通信、状态、安全、观测和多租户治理全部做成可长期演进的基础设施。

