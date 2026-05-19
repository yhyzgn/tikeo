# scheduler 开发阶段总提示词（AI 接手协议）

> 适用对象：Codex、Claude、Gemini、OpenCode、Cursor Agent、Aider、Devin 类 AI 编程智能体。  
> 目标：任何智能体在任意开发阶段打开本仓库后，都能基于本文件、`./design`、`./.memory`、`./.prompt` 无缝接手后续工作，保证上下文不丢失、实现可验证、提交可追溯。

---

## 0. 最高优先级工作规则

1. **先读上下文，再开发**：开始任何任务前必须阅读：
   - `./prompt.md`（本文件）
   - `./design/scheduler-architecture-design.md`（架构与产品设计源）
   - `./.memory/README.md`
   - `./.memory/session-log.md`
   - `./.memory/decisions.md`
   - `./.memory/progress.md`
   - `./.memory/next.md`
   - `./.prompt/README.md`
   - `./.prompt/` 中编号最新的阶段提示词
2. **上下文永不丢失**：每次推进工作后，都必须更新 `./.memory`；每个阶段完成或调整后，都必须更新 `./.prompt` 中后续阶段提示词。
3. **路线图完成项必须回写设计文档**：每个开发工作项完成后，必须在 `./design/scheduler-architecture-design.md` 的开发路线图中把对应条目标记为完成 `✅` / `[x]`。
4. **代码必须可验证**：每个开发任务都要执行编译、测试、运行/冒烟验证。全部通过后才能提交。
5. **自动提交并推送**：验证通过后自行 `git commit` 并 `git push` 到远程仓库。若远程不存在或推送失败，必须在 `./.memory/session-log.md` 和最终回复中明确记录原因与下一步。
6. **保持小步提交**：每个提交聚焦一个阶段或一个可验证能力，避免混杂大改。
7. **不丢设计目标**：实现必须服从 `./design/scheduler-architecture-design.md`，若代码实现需要偏离设计，必须先更新设计文档和 `./.memory/decisions.md`。
8. **Rust 代码必须 workspace + crates 解耦**：整个 Rust 项目必须使用 Cargo workspace；后端主程序入口位于仓库根 `src/main.rs`；其余 Rust 模块抽取为独立 crate 并统一放在 `./crates/` 下，禁止把大量业务模块堆在单一 crate 中。
9. **Web 端必须独立在 `./web/`**：Web 管理端代码必须放在 `./web/` 下，使用 React + TypeScript + Ant Design，包管理器固定使用 Bun。禁止使用 `webui/` 作为新的前端目录。
10. **不要让 Worker 暴露入站端口**：scheduler 的核心架构是 Worker 主动通过 gRPC/HTTP2 tunnel 连接 Server，Server 反向指令复用该长连接。
11. **Server 不执行用户代码**：动态脚本、WASM、HTTP、SQL 等处理器必须由 Worker 侧受控环境执行，Server 只调度、治理、审计。
12. **依赖库尽量使用最新版**：新增 Rust crate、前端 npm/bun 包、构建工具和运行时依赖时，默认选择当前最新稳定版；若不能使用最新版，必须在 `./.memory/decisions.md` 记录原因、锁定版本和升级条件。
13. **HTTP 业务接口必须统一返回 `{code,message,data}`**：`code` 是成功判断标准，整数 `0` 表示成功，非 0 表示失败；`message` 是响应信息；`data` 是响应数据，即使为 `null` 也必须显式返回。

---

## 1. 项目简要说明

`scheduler` 是一个用 Rust 从零构建的分布式任务调度与计算平台，目标是成为企业内部公共任务调度基础设施。

核心设计目标：

- Rust 原生、内存安全、异步优先。
- 单二进制启动，内置 Web UI。
- Server / Worker 均容器优先，绝对支持 K8s、Docker、Compose、跨集群、跨 VPC 部署。
- Worker 主动建立 gRPC/HTTP2 双向 tunnel，注册、心跳、任务分发、取消、日志、证书轮换都走同一连接。
- Server 不直连 Worker，不要求 Worker 暴露入站端口。
- 支持 CRON、FIX_RATE、FIX_DELAY、API 触发、延迟任务、一次性任务、日历调度。
- 支持单机、广播、分片、Map、MapReduce、DAG 工作流、长运行任务。
- 支持多语言 SDK：Rust、Go、Python、Java、Node.js。
- 支持多语言动态脚本：Shell、Python、Node.js/TypeScript、PowerShell、Rhai/CEL/JSONLogic、WASM/WASI、容器化 Runner。
- 支持安全沙箱、RBAC、OIDC、mTLS、审计、Secret reference、URL policy、参数化 SQL。
- 提供 Web UI 管理控制台和 HTTP/OpenAPI 管理接口。

详细设计以 `./design/scheduler-architecture-design.md` 为准。

---

## 2. 推荐技术栈

后续实现应优先遵循设计文档中的技术选型；依赖版本默认选择当前最新稳定版，避免引入已停止维护、长期不更新或存在已知安全风险的库：

- Rust 2024 Edition
- Tokio：异步运行时
- Tonic / Prost：gRPC 与 protobuf
- Axum：HTTP REST API / Web UI 静态资源托管
- SeaORM：SQLite / MySQL / PostgreSQL / CockroachDB 抽象
- openraft：Server 集群共识
- Wasmtime：WASM 沙箱
- tracing / metrics / OpenTelemetry：日志、指标、追踪
- Clap：CLI
- config-rs：配置
- Serde：JSON/TOML/YAML
- include_dir：嵌入前端静态资源
- OpenAPI：优先 `utoipa` / `aide` / `schemars` 等稳定方案

---

## 3. Web UI 与 UI 资源库说明

Web UI 是平台默认管理入口，必须作为一等能力实现。

### 3.1 UI 功能模块

至少规划并逐步实现：

- Dashboard：平台健康、调度趋势、失败率、延迟、Worker 在线率。
- Jobs：任务列表、详情、创建/编辑、版本历史、调度仿真。
- Instances：实例列表、详情、attempt、实时日志、重试、取消。
- Workflows：DAG 可视化编辑、YAML/JSON 双模式、dry-run、回放。
- Workers：Worker 列表、Worker Pool、连接详情、capabilities、drain。
- Scripts：多语言脚本编辑、版本、审批、沙箱策略、执行记录。
- Apps & Tenants：租户、namespace、app、quota、标签。
- Secrets：Secret reference、轮换、使用关系，不展示明文。
- Alerts：告警规则、通知渠道、静默、升级。
- Audit：审计日志、过滤、导出。
- Settings：OIDC、RBAC、API Token、系统配置、集群节点。

### 3.2 UI 工程建议

Web 端技术栈固定为 **React + TypeScript + Vite + Ant Design**，代码目录固定为 `./web/`，包管理工具固定为 **Bun**。原则：

- API client 从 OpenAPI 生成，避免手写漂移。
- 表单尽可能从 JSON Schema / OpenAPI 元数据生成。
- 实时能力优先 SSE，必要时使用 WebSocket。
- 浏览器不得直接访问 Worker。
- UI 不保存长期凭证。
- Gateway 统一处理 CSRF、CSP、安全响应头、Token 刷新。
- 支持暗色模式、基础 a11y、键盘操作、中文/英文。

### 3.3 UI 资源库

若仓库尚未创建 UI 资源库，后续智能体应在实现 Web UI 时建立：

```text
web/
├── src/
│   ├── app/              # 应用入口、路由、布局
│   ├── components/       # 通用组件
│   ├── features/         # jobs / instances / workflows / workers / scripts 等模块
│   ├── api/              # OpenAPI 生成或封装后的 client
│   ├── styles/           # theme、tokens、全局样式
│   ├── i18n/             # zh-CN / en-US
│   └── assets/           # 图标、插图、静态资源
├── public/
├── package.json
└── vite.config.ts
```

如果需要组件库，优先选择成熟、维护良好、易定制、TypeScript 友好的方案。不得为了 UI 引入过重或维护停滞的依赖。

---

## 4. 开发阶段推进协议

每个阶段必须遵循以下循环：

```text
读取上下文
  -> 确认当前阶段目标
  -> 更新或创建 ./.prompt/<NN>-<phase>.md
  -> 实现最小可验证切片
  -> cargo fmt
  -> cargo clippy / lint
  -> cargo test
  -> cargo build
  -> 运行/冒烟验证
  -> 更新 ./.memory
  -> git status 检查
  -> git add
  -> git commit
  -> git push
  -> 最终回复说明证据、提交 hash、推送状态、下一阶段
```

如果前端存在，还必须执行对应命令，例如：

```bash
bun install
bun run lint
bun run typecheck
bun test
bun run build
```

如果工作区命令尚未存在，需要先创建合理的脚本，并在 `./.memory/commands.md` 中记录。新增依赖后应运行依赖树/安全检查；若暂未配置安全检查，也要在 `.memory/risks.md` 记录。

---

## 5. 编译、测试、运行要求

每次开发任务至少执行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
```

如果项目刚初始化，命令应随实际 workspace 调整，但必须记录到 `./.memory/commands.md`。

运行/冒烟验证示例：

```bash
cargo run --bin scheduler -- serve --config examples/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json
```

若服务暂未实现，应运行当前阶段等价冒烟命令，并在 memory 中说明替代验证。

---

## 6. Git 提交与推送规范

验证全部通过后必须提交并推送。

提交信息要求：

- message 尽可能丰富、有层次。
- 可以使用必要 emoji / sticker 图标增强可读性。
- 第一行说明“为什么做这个变更”，不是简单罗列文件。
- 正文说明设计依据、主要改动、验证结果、风险与后续。
- 必须包含测试证据。

推荐格式：

```text
🚀 建立 scheduler 调度核心的可验证基础骨架

Context:
- 基于 design/scheduler-architecture-design.md 的 Phase 1 目标推进
- 本阶段聚焦 workspace、配置、HTTP healthz 与基础 CLI

Changes:
- 初始化 Rust workspace 与 server crate
- 增加 Axum healthz/readyz 路由
- 增加配置加载与 CLI serve 子命令

Verification:
- cargo fmt --all -- --check ✅
- cargo clippy --workspace --all-targets --all-features -- -D warnings ✅
- cargo test --workspace --all-features ✅
- cargo build --workspace --all-features ✅
- curl http://127.0.0.1:9090/healthz ✅

Memory:
- Updated .memory/progress.md
- Updated .memory/commands.md
- Added .prompt/002-...

Risk:
- No database migration yet
- OpenAPI route planned next

✨ Generated with AI pair development handoff protocol
```

如果仓库配置了 oh-my-codex Lore Commit Protocol，则同时满足其 trailer 要求，例如：

```text
Constraint: design/scheduler-architecture-design.md Phase 1
Confidence: high
Scope-risk: narrow
Tested: cargo fmt; cargo clippy; cargo test; cargo build; healthz smoke
Not-tested: cross-cluster worker tunnel pending later phase
```

推送：

```bash
git push
```

若当前分支无 upstream：

```bash
git push -u origin HEAD
```

若远程不存在、无凭证或网络失败，不要伪造成功；记录到 `./.memory/session-log.md`。

---

## 7. 记忆库协议：`./.memory`

所有智能体必须把持续上下文写入 `./.memory`。

目录结构：

```text
.memory/
├── README.md          # 记忆库使用说明
├── project.md         # 项目目标、架构摘要、不可破坏约束
├── decisions.md       # 已确认设计/技术决策
├── progress.md        # 阶段进度、已完成/进行中/待办
├── commands.md        # 构建、测试、运行、调试命令
├── session-log.md     # 每次会话/任务推进日志
├── next.md            # 下一步明确任务
└── risks.md           # 风险、阻塞、技术债、验证缺口
```

每次任务结束必须更新：

- `session-log.md`：记录日期、智能体、任务、改动、验证、提交、推送结果。
- `progress.md`：更新完成状态。
- `next.md`：写清下一阶段入口任务。
- `commands.md`：若新增或修正命令，必须更新。
- `decisions.md`：若发生设计/技术决策，必须更新。
- `risks.md`：若有风险或未验证项，必须更新。

---

## 8. 阶段提示词协议：`./.prompt`

`./.prompt` 用来保存后续阶段工作提示词，使新窗口或新智能体能直接接续。

文件命名：

```text
.prompt/
├── README.md
├── 001-bootstrap.md
├── 002-http-api-and-openapi.md
├── 003-worker-tunnel.md
├── 004-storage-and-scheduler.md
└── ...
```

规则：

1. 每个阶段开始前，创建或更新对应编号提示词。
2. 每个阶段完成后，必须检查后续阶段提示词是否仍然正确。
3. 如果当前阶段返工、调整架构或改变技术方案，必须同步更新所有受影响的后续 prompt。
4. 每个阶段 prompt 必须包含：
   - 阶段目标
   - 设计依据
   - 当前上下文入口
   - 具体任务列表
   - 验证命令
   - 提交要求
   - 完成后应更新哪些 memory 文件
   - 下一阶段建议

---

## 9. 初始推荐阶段划分

后续智能体可根据实际进度调整，但调整必须写入 `./.memory/decisions.md` 与后续 `./.prompt`。

1. **001-bootstrap**：Rust workspace、`./crates/` 解耦 crate 拆分、CI、本地命令、基础配置。
2. **002-http-api-and-openapi**：Axum gateway、healthz/readyz、基础 REST、OpenAPI。
3. **003-worker-tunnel**：gRPC protobuf、Worker 主动注册、心跳、连接路由表。
4. **004-storage-and-scheduler**：SeaORM、SQLite/MySQL、Job/Instance 模型、CRON/FIX_RATE。
5. **005-worker-sdk-rust**：Rust Worker SDK、任务执行、状态上报、日志流。
6. **006-web-ui-foundation**：`./web/` React + Ant Design + Bun 前端工程、登录壳、Dashboard、Job 列表。
7. **007-dynamic-script-sandbox**：Script Processor、安全策略、资源限制、审计。
8. **008-workflow-engine**：DAG、条件、上下文、可视化接口。
9. **009-container-and-k8s**：Dockerfile、Compose、Helm Chart、跨集群 Worker 示例。
10. **010-hardening-and-observability**：RBAC/OIDC/mTLS、Prometheus、OTLP、审计、告警。

---

## 10. 接手时的第一条执行指令

任何新智能体接手时，请按以下顺序执行：

```bash
pwd
find . -maxdepth 3 -type f | sort
cat ./prompt.md
cat ./design/scheduler-architecture-design.md
cat ./.memory/README.md
cat ./.memory/progress.md
cat ./.memory/next.md
ls -la ./.prompt
```

然后读取最新阶段 prompt，继续推进，不要重新发明上下文。

---

## 11. 本提示词自身维护

如果项目规则变化，必须更新本文件，并在：

- `./.memory/decisions.md`
- `./.memory/session-log.md`
- 受影响的 `./.prompt/*.md`

中记录变更原因。
