# 021-phase2-workflow-and-queue-foundation

## 背景

020 已完成 015-019 的安全与质量善后：移除静态 admin token、修复脚本版本快照、补基础 metrics、收紧告警 webhook、修复 Web 质量门禁。021 必须优先进入 `design/scheduler-architecture-design.md` 中 Phase 2「工作流与分布式」工作项，不再把 Phase 3 的 RBAC/企业治理作为当前阶段主线。

## 阶段目标

交付 Phase 2 的第一条可运行纵切：**DAG 工作流引擎 + 队列/延迟队列基础 + 实时事件流骨架**。目标不是一次性完成所有 Phase 2 大项，而是先建立后续 Map/MapReduce、子工作流、工作流编辑器、SSE/WebSocket、分布式存储/集群的核心数据结构和执行链路。

## 必做范围

### 1. DAG 工作流引擎基础

- 增加 workflow / workflow_node / workflow_edge / workflow_instance / workflow_node_instance 存储模型。
- 全库继续禁止外键，全部使用字段软关联 + repository/service 校验。
- 支持 DAG 定义校验：节点唯一、边引用存在、禁止环、必须有 start node。
- 支持最小条件分支：edge condition 先以表达式字符串保存，021 可实现 `always` / `on_success` / `on_failure` 三类基础条件。
- 提供 HTTP API，响应必须保持 `{ code, message, data }`，且 `data` 必须出现：
  - `POST /api/v1/workflows`
  - `GET /api/v1/workflows`
  - `GET /api/v1/workflows/:id`
  - `POST /api/v1/workflows/:id/validate`
  - `POST /api/v1/workflows/:id/run`
  - `GET /api/v1/workflow-instances/:id`

### 2. 任务队列与持久化延迟队列基础

- 引入 queued/delayed dispatch 概念，避免 scheduler tick 直接强耦合 worker dispatch。
- 增加 dispatch queue 存储模型，字段至少覆盖：id、job_instance_id/workflow_node_instance_id（二选一软关联）、priority、run_after、status、attempt、worker_selector、created_at、updated_at。
- 调度器将待执行任务写入队列；dispatcher loop 从队列按 `run_after <= now` + priority 拉取。
- 021 可先支持单 server 进程内竞争保护，但数据库结构要为后续多 server/raft 做准备。

### 3. 实时事件流骨架

- 增加统一 instance event 记录或内存 broadcast abstraction，用于后续 SSE/WebSocket 和 gRPC Server Stream。
- 先提供 SSE HTTP 接口骨架：
  - `GET /api/v1/events/instances/:id/stream`
- 事件至少覆盖 instance started/succeeded/failed、workflow node started/succeeded/failed、log appended。
- 若 021 时间不足，可先实现后端 SSE + Web API client，不强制完整 UI。

### 4. Web UI 最小入口

- 新增 Workflows 菜单与列表页。
- 支持创建/查看 workflow JSON/YAML 定义的基础页面。
- 可先不做完整可视化拖拽编辑器；可视化编辑器作为 022/023 后续。

### 5. 文档、记忆与路线图

- 更新 `design/scheduler-architecture-design.md`：021 完成的 Phase 2 子项使用 `[x]` 标记，部分完成则拆成子项并准确标记。
- 更新 `.memory/*`，记录 021 的架构决策、命令、风险和下一步。
- 根据实际完成情况生成/更新 `.prompt/022-*.md`。若 021 回滚或调整，必须同步更新后续 prompt。

## 非目标

- 暂不做 Phase 3 的完整 RBAC permission/resource/action 改造。
- 暂不做 Swagger UI（禁止）。
- 暂不做真实 Raft 共识集群；021 只为多 server 做数据结构与边界准备。
- 暂不做完整图形化 DAG 拖拽编辑器；021 先完成数据/API/执行骨架。
- 暂不引入数据库外键。

## 质量门禁

每次阶段推进后必须运行并通过：

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `mvn -f java/pom.xml -q test`
- `bun run --cwd web lint`
- `bun run --cwd web typecheck`
- `bun test --cwd web`
- `bun run --cwd web build`
- `docker compose config`

若改动涉及运行时链路，还要补充本地 smoke：server 启动、healthz、登录、workflow validate/run、事件流连接。

## 提交要求

- 提交前确认没有误提交 OMX 生成噪音文件，例如根目录未跟踪 `AGENTS.md`。
- commit message 使用 Lore 协议并包含 OmX co-author trailer。
- 验证全部通过后提交并推送远程。
