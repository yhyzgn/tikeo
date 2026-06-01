# Server + Web + Java SDK/Demo 联合自动化测试落地计划与状态表

> 生成日期：2026-06-01  
> 适用范围：tikee server、web 控制台、Java SDK、Java Spring worker demo、storage 数据库矩阵。  
> 口径：每个测试项必须同时包含“执行命令/步骤、断言标准、证据产物、测试结果、状态”。状态仅允许：`通过` / `待执行` / `失败` / `阻塞` / `跳过`。  
> 当前已实测证据来自最近一次本地执行；未实测项不得标记为通过。

## 1. 总体目标

1. 验证 server、web、Java SDK、Java Spring demo 能按真实业务链路联调。
2. 验证任务创建、触发、worker 注册、dispatch、执行日志、实例状态、workflow 节点推进等核心功能符合预期。
3. 验证 storage 在 SQLite、PostgreSQL、MySQL 稳定版本上可迁移、可写入、可读回、可幂等启动。
4. 所有自动化结果落盘，便于人工 review 和 CI 回放。

## 2. 通用环境准备

### 2.1 工具要求

| 项 | 要求 | 检查命令 | 当前测试结果 | 状态 |
| --- | --- | --- | --- | --- |
| Rust toolchain | 项目 workspace 可编译 | `cargo --version` | 未单独记录版本；`cargo check --workspace` 已通过 | 通过 |
| Cargo fmt | 格式检查可运行 | `cargo fmt --all -- --check` | 已通过 | 通过 |
| Docker | 数据库矩阵 smoke 需要 | `docker info` | 已可用，并成功启动 PostgreSQL 16 / MySQL 8.4 | 通过 |
| PostgreSQL | 13+，本地资产用 16 | `./scripts/db-compat-smoke.sh` | PostgreSQL 16 smoke 已通过 | 通过 |
| MySQL | 8.0+ / 8.4 LTS，utf8mb4 | `./scripts/db-compat-smoke.sh` | MySQL 8.4 smoke 已通过 | 通过 |
| Bun | Web 测试/构建 | `cd web && bun --version` | 本轮未执行 | 待执行 |
| JDK / Gradle | Java SDK/demo 测试 | `java -version` / `./gradlew test` | 本轮未执行 | 待执行 |
| curl/python3 | smoke 脚本依赖 | `curl --version && python3 --version` | 本轮未单独执行 | 待执行 |
| Chromium/Playwright | Web live/e2e 截图验收 | 以脚本或 Playwright 配置为准 | 本轮未执行 | 待执行 |

### 2.2 推荐报告目录

```bash
export TIKEE_E2E_RUN_ID="joint-$(date -u +%Y%m%dT%H%M%SZ)-$$"
export TIKEE_E2E_REPORT_DIR="$PWD/.dev/reports/$TIKEE_E2E_RUN_ID"
mkdir -p "$TIKEE_E2E_REPORT_DIR"
```

### 2.3 一键命令顺序

```bash
# A. 基础代码质量
rtk cargo fmt --all -- --check
rtk cargo check --workspace
rtk cargo test -p tikee-storage

# B. 数据库兼容性矩阵
rtk bash scripts/db-compat-smoke.sh

# C. Server 全工作区回归
rtk cargo clippy --workspace --all-targets --all-features -- -D warnings
rtk cargo test --workspace --all-features -- --test-threads=1

# D. Web 回归
rtk bash -lc 'cd web && bun install --frozen-lockfile'
rtk bash -lc 'cd web && bun test'
rtk bash -lc 'cd web && bun run typecheck'
rtk bash -lc 'cd web && bun run lint'
rtk bash -lc 'cd web && bun run build'

# E. Java SDK / Demo 回归
rtk bash -lc 'cd sdks/java && ./gradlew test --no-daemon'
rtk bash -lc 'cd examples/java/spring-worker-demo && ./gradlew test --no-daemon'

# F. Server + Java demo live smoke
rtk bash deploy/smoke/java-demo-integration-smoke.sh

# G. Server + Web live smoke
rtk bash deploy/smoke/web-live-smoke.sh

# H. Server + Web + 双 Java demo e2e
rtk bash deploy/smoke/server-web-java-joint-e2e.sh
```

## 3. P0-A：静态、编译、单元与存储矩阵

| ID | 测试项 | 执行命令 | 核心断言 | 证据产物 | 当前测试结果 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| A-SRV-001 | Rust 格式检查 | `rtk cargo fmt --all -- --check` | 退出码为 0，无格式 diff | 终端日志 | 已执行通过 | 通过 |
| A-SRV-002 | Rust workspace 编译检查 | `rtk cargo check --workspace` | workspace 全部 crate check 通过 | 终端日志 | 已执行通过 | 通过 |
| A-SRV-003 | Storage 单元测试 | `rtk cargo test -p tikee-storage` | storage 33 个单测 + database_compat 2 个测试通过 | 终端日志 | 已执行通过，33 + 2 通过 | 通过 |
| A-DB-001 | SQLite storage smoke | `rtk cargo test -p tikee-storage --test database_compat sqlite_database_compatibility_smoke -- --nocapture` | SQLite 空 schema 迁移、幂等迁移、scope/job/plugin/script/instance/log 断言通过 | 终端日志 | 已执行通过 | 通过 |
| A-DB-002 | PostgreSQL storage smoke | `rtk bash scripts/db-compat-smoke.sh` | PostgreSQL 16 上迁移与 CRUD smoke 通过 | 终端日志、Docker 服务状态 | 已执行通过 | 通过 |
| A-DB-003 | MySQL storage smoke | `rtk bash scripts/db-compat-smoke.sh` | MySQL 8.4 上迁移、复合索引、text/json、Unicode 日志 smoke 通过 | 终端日志、Docker 服务状态 | 已执行通过 | 通过 |
| A-SRV-004 | Rust clippy | `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` | 无 warning/error | CI/终端日志 | 本轮未执行 | 待执行 |
| A-SRV-005 | Rust 全 workspace 测试 | `rtk cargo test --workspace --all-features -- --test-threads=1` | server/storage/proto 等全量测试通过；关键状态机断言通过 | CI/终端日志 | 本轮未执行 | 待执行 |
| A-SRV-006 | Server raft targeted 测试 | `rtk cargo test -p tikee-server raft -- --nocapture` | Raft metadata/member/log/snapshot 相关测试通过 | CI/终端日志 | 本轮未执行 | 待执行 |
| A-SRV-007 | Worker targeted 测试 | `rtk cargo test -p tikee-server worker -- --nocapture` | worker registry、session、master/fencing 相关测试通过 | CI/终端日志 | 本轮未执行 | 待执行 |
| A-WEB-001 | Web Vitest | `rtk bash -lc 'cd web && bun test'` | 路由守卫、API client、字段映射、表单 payload 测试通过 | Bun test log | 本轮未执行 | 待执行 |
| A-WEB-002 | Web typecheck | `rtk bash -lc 'cd web && bun run typecheck'` | TypeScript 无错误 | Bun log | 本轮未执行 | 待执行 |
| A-WEB-003 | Web lint | `rtk bash -lc 'cd web && bun run lint'` | ESLint 无错误 | Bun log | 本轮未执行 | 待执行 |
| A-WEB-004 | Web build | `rtk bash -lc 'cd web && bun run build'` | 生产构建成功，SPA fallback 不破坏构建 | build log / dist | 本轮未执行 | 待执行 |
| A-JAVA-001 | Java SDK 单元测试 | `rtk bash -lc 'cd sdks/java && ./gradlew test --no-daemon'` | management client、API-Key、worker client、请求 payload 断言通过 | Gradle report | 本轮未执行 | 待执行 |
| A-JAVA-002 | Java worker client targeted 测试 | `rtk bash -lc 'cd sdks/java && ./gradlew :tikee:test --tests com.yhyzgn.tikee.worker.client.GrpcTikeeWorkerClientTest --no-daemon'` | gRPC 注册、心跳、任务响应协议测试通过 | Gradle report | 本轮未执行 | 待执行 |
| A-DEMO-001 | Java Spring demo 单元测试 | `rtk bash -lc 'cd examples/java/spring-worker-demo && ./gradlew test --no-daemon'` | demo processors、Spring 配置、任务用例类测试通过 | Gradle report | 本轮未执行 | 待执行 |

## 4. P0-B：Server + Java demo 集成 smoke

执行入口：

```bash
export TIKEE_INTEGRATION_REPORT_DIR="$PWD/.dev/reports"
rtk bash deploy/smoke/java-demo-integration-smoke.sh
```

| ID | 测试项 | 自动化步骤 | 核心断言 | 证据产物 | 当前测试结果 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| B-BOOT-001 | 临时 server 启动 | 脚本生成临时 config，启动 server | `GET /readyz` 成功；端口隔离；DB 使用临时文件 | server log、config | 本轮未执行 | 待执行 |
| B-AUTH-001 | 初始化管理员注册/登录 | 查询 bootstrap 状态，注册或登录 | 返回 `data.token`；bootstrap 注册后关闭 | auth response JSON | 本轮未执行 | 待执行 |
| B-WORKER-001 | Java demo 启动 | 脚本启动 `bootRun` | `/demo/health` 成功 | java-demo log | 本轮未执行 | 待执行 |
| B-WORKER-002 | Worker 注册在线 | 查询 `/api/v1/workers` | `spring-demo-worker` online；namespace/app/cluster/region 正确 | workers JSON | 本轮未执行 | 待执行 |
| B-WORKER-003 | Worker 结构化能力 | 查询 worker capabilities/processors | processorNames、pluginProcessors、script runtime 字段结构化返回，不靠字符串约定 | workers JSON | 本轮未执行 | 待执行 |
| B-WORKER-004 | Worker master/election 字段 | 查询 `/api/v1/workers` | `master.domain/isMaster/masterWorkerId/term/fencingToken` 存在且语义正确 | workers JSON | 本轮未执行 | 待执行 |
| B-JOB-001 | API single 成功任务 | 创建并触发 `demo.echo` | instance `succeeded`；执行 worker 合法；日志包含 echo 业务输出 | report JSON、instance JSON、logs JSON | 本轮未执行 | 待执行 |
| B-JOB-002 | API failure 任务 | 创建并触发 `demo.fail` | instance `failed`；错误 message/log 持久化 | report JSON、logs JSON | 本轮未执行 | 待执行 |
| B-JOB-003 | Broadcast 任务 | 创建并触发 `demo.context` broadcast | parent/attempt 成功；至少一个 worker attempt 有日志 | report JSON | 本轮未执行 | 待执行 |
| B-JOB-004 | Fixed-rate 任务 | 创建 fixed_rate `demo.heartbeat` | 至少一个调度实例自动生成并成功 | report JSON、instances JSON | 本轮未执行 | 待执行 |
| B-JOB-005 | Cron 任务 | 创建 cron `demo.report` | 至少一个 cron 实例生成并成功 | report JSON、instances JSON | 本轮未执行 | 待执行 |
| B-SCRIPT-001 | Shell 脚本任务 | 创建 shell script job 并触发 | worker 沙箱执行；stdout 进入 worker 控制台和实例日志 | java-demo log、instance logs | 本轮未执行 | 待执行 |
| B-SCRIPT-002 | Python/JS/TS/Rhai 脚本任务 | 逐语言创建/触发脚本任务 | 不因 processor capability 字符串缺失卡 pending；失败时给出明确治理原因 | report JSON、logs JSON | 本轮未执行 | 待执行 |
| B-WF-001 | Workflow job 节点 | 创建 workflow，run/materialize | workflow instance 最终 `succeeded`，节点状态与任务实例一致 | workflow JSON、node JSON | 本轮未执行 | 待执行 |
| B-LOG-001 | 实例日志持久化 | 查询 `/api/v1/instances/{id}/logs` | demo 执行日志存在、workerId 可见、无重复爆量 | logs JSON | 本轮未执行 | 待执行 |
| B-QUEUE-001 | 队列无堵塞 | smoke 完成后查 queue overview | 无长期 `pending/queued` 积压；历史失效 worker 不阻塞新任务 | queue JSON | 本轮未执行 | 待执行 |

## 5. P0-C：Server + Web 联合验证

执行入口：

```bash
rtk bash deploy/smoke/web-live-smoke.sh
```

| ID | 测试项 | 自动化步骤 | 核心断言 | 证据产物 | 当前测试结果 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| C-WEB-001 | Web dev server 启动 | 启动/复用 server，启动 web | Web 可访问；API base 指向测试 server；无 console error | web log、console log | 本轮未执行 | 待执行 |
| C-WEB-002 | 根路径重定向 | 登录后访问 `/` | 自动进入 `/dashboard` | URL assert、screenshot | 本轮未执行 | 待执行 |
| C-WEB-003 | 登录态路由守卫 | 登录后访问 `/login` | 跳过 login，返回 dashboard/目标页 | screenshot、URL assert | 本轮未执行 | 待执行 |
| C-WEB-004 | 刷新二级路由 | 刷新 `/jobs/:id/topology`、`/workflows/:id/designer` 等 | 不 404；SPA fallback 正常 | HTTP status、screenshot | 本轮未执行 | 待执行 |
| C-WEB-005 | Worker 列表展示 | 打开 Workers 页 | API 字段与 UI 一致；Capabilities 不误列 processor | screenshot、workers JSON | 本轮未执行 | 待执行 |
| C-WEB-006 | Task 列表分页 | 打开任务列表，切换 page size | 默认 20；下拉选择可用；cookie 持久化 | screenshot、cookie dump | 本轮未执行 | 待执行 |
| C-WEB-007 | Task 新建/编辑抽屉 | 创建/编辑 api、cron、fixed_rate、script job | 抽屉宽度、字段、processor/script 选择逻辑正确；payload 小驼峰 | screenshot、request payload | 本轮未执行 | 待执行 |
| C-WEB-008 | 操作按钮样式 | 检查任务/实例操作栏 | 按钮平铺，主色调跟随全局主题 | screenshot | 本轮未执行 | 待执行 |
| C-WEB-009 | 调度日历维护 | 新建/编辑 calendar | 维护/冻结窗口使用范围选择标签交互，不手写 JSON | screenshot、payload | 本轮未执行 | 待执行 |
| C-WEB-010 | 实例详情日志 | 打开 instance detail | 非广播展示“执行器”；广播展示子执行；worker/status/updatedAt 可见 | screenshot、logs JSON | 本轮未执行 | 待执行 |
| C-WEB-011 | Workflow designer 节点属性 | 编辑 job 节点 | 绑定调度任务后 processor 不手填，来自任务绑定 | screenshot、payload | 本轮未执行 | 待执行 |
| C-WEB-012 | GitOps / IaC 页面 | 打开相关功能入口 | Manifest export/diff 可用；Terraform provider 若未 live 验证不得显示已完成 | screenshot、API response | 本轮未执行 | 待执行 |

## 6. P0-D：Server + Web + 双 Java demo 端到端

执行入口：

```bash
rtk bash deploy/smoke/server-web-java-joint-e2e.sh
```

| ID | 测试项 | 自动化步骤 | 核心断言 | 证据产物 | 当前测试结果 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| D-BOOT-001 | 隔离 server 启动 | 临时 config：HTTP 19090、tunnel 19998、临时 DB | `/readyz` 成功 | config、server log | 本轮未执行 | 待执行 |
| D-BOOT-002 | Java demo A 启动 | 端口 18080，固定 client instance | `/demo/health` 成功 | demo A log | 本轮未执行 | 待执行 |
| D-BOOT-003 | Java demo B 启动 | 端口 18081，同 election domain | `/demo/health` 成功 | demo B log | 本轮未执行 | 待执行 |
| D-ELECT-001 | 同 domain 唯一 master | 查询 workers | 同 domain 仅 1 个 `isMaster=true` | workers-before JSON | 本轮未执行 | 待执行 |
| D-DISP-001 | Single 任务优先 master | 触发 `demo.echo` single | 成功实例执行 worker 等于当前 master | instance JSON、logs JSON | 本轮未执行 | 待执行 |
| D-DISP-002 | Broadcast 发给全部 worker | 触发 `demo.context` broadcast | 两个 worker 均有 attempt/日志 | instance JSON、attempts/logs | 本轮未执行 | 待执行 |
| D-FAILOVER-001 | master 停止后 follower 晋升 | kill master demo | 另一个 worker 成为 master；term/fencing 更新 | workers timeline JSON | 本轮未执行 | 待执行 |
| D-FAILOVER-002 | failover 后 single 成功 | 再触发 `demo.echo` | 新 master 执行成功，无 stale worker 阻塞 | instance JSON、queue JSON | 本轮未执行 | 待执行 |
| D-WEB-001 | Web Worker 页展示切换 | failover 前后截图 | UI Master/Follower 状态随 API 改变 | screenshots | 本轮未执行 | 待执行 |
| D-WEB-002 | Web 实例详情日志一致 | 打开 failover 后实例详情 | UI 日志与 API logs 一致 | screenshot、logs JSON | 本轮未执行 | 待执行 |

## 7. P1-E：SDK Management / API-Key 联合验证

| ID | 测试项 | 执行方式 | 核心断言 | 证据产物 | 当前测试结果 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| E-KEY-001 | Service Account 创建 | API/Web 创建 SA | SA 绑定 namespace/app/workerPool，状态 active | API JSON、screenshot | 本轮未执行 | 待执行 |
| E-KEY-002 | SDK API-Key 创建 | API/Web 创建 key | key 格式正确，只在创建弹窗显示明文 | API JSON、screenshot | 本轮未执行 | 待执行 |
| E-KEY-003 | 列表脱敏 | 打开 API-Key 列表 | 中间脱敏，两端明文，无复制脱敏值误导 | screenshot | 本轮未执行 | 待执行 |
| E-KEY-004 | key 元数据编辑 | 编辑名称/作用域/有效期 | key 值不变，元数据更新 | API JSON、audit JSON | 本轮未执行 | 待执行 |
| E-KEY-005 | Java management client 用 key | Java SDK management 测试 | 授权 scope 可调用，越权失败 | Gradle report、API response | 本轮未执行 | 待执行 |
| E-KEY-006 | 审计记录 | 查询 audit logs | SA/key create/update/revoke/use 均有审计 | audit JSON | 本轮未执行 | 待执行 |
| E-KEY-007 | 禁用 SA 级联 | 禁用 SA 后使用旧 key | 关联 active key 被吊销，旧 key 调用失败 | API JSON、audit JSON | 本轮未执行 | 待执行 |

## 8. P1-F：脚本沙箱与插件任务联合验证

| ID | 测试项 | 执行方式 | 核心断言 | 证据产物 | 当前测试结果 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| F-SCRIPT-001 | Shell 脚本沙箱 | 创建/触发 shell script job | 默认沙箱执行，stdout 进 worker 控制台和实例日志 | demo log、logs JSON | 本轮未执行 | 待执行 |
| F-SCRIPT-002 | Python 脚本治理 | 创建/触发 python script job | 若运行器不可用，明确 fail-closed；不可 pending 卡死 | instance/logs JSON | 本轮未执行 | 待执行 |
| F-SCRIPT-003 | JavaScript 脚本治理 | 创建/触发 JavaScript job | 运行或治理失败都有终态与日志 | instance/logs JSON | 本轮未执行 | 待执行 |
| F-SCRIPT-004 | TypeScript 脚本治理 | 创建/触发 TypeScript job | 运行或治理失败都有终态与日志 | instance/logs JSON | 本轮未执行 | 待执行 |
| F-SCRIPT-005 | Rhai 输出 | 触发 rhai script job | print 输出进入 worker 控制台与实例日志 | demo log、logs JSON | 本轮未执行 | 待执行 |
| F-SANDBOX-001 | Wasmtime/SRT 自动选择 | demo 启动并触发脚本 | Auto/default 优先 wasm/wasmtime；缺失时安装或明确失败 | demo log | 本轮未执行 | 待执行 |
| F-PLUGIN-001 | 插件注册 | 创建 plugin processor/alert channel | 结构化保存，非字符串拼接约定 | API JSON | 本轮未执行 | 待执行 |
| F-PLUGIN-002 | 插件类型任务创建 | Web/API 创建插件任务 | 候选项来自 worker/plugin 结构化注册 | screenshot、payload | 本轮未执行 | 待执行 |
| F-PLUGIN-003 | 插件任务执行日志 | 触发插件任务 | processor 输出进实例日志；失败有清晰原因 | logs JSON、demo log | 本轮未执行 | 待执行 |

## 9. P2-G：GitOps / IaC / Terraform / K8s 验证

| ID | 测试项 | 执行方式 | 核心断言 | 证据产物 | 当前测试结果 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| G-GITOPS-001 | Manifest 导出 | `GET /api/v1/gitops/manifest` | YAML/JSON 可解析，有 checksum | manifest file | 本轮未执行 | 待执行 |
| G-GITOPS-002 | Manifest diff | `POST /api/v1/gitops/diff` | 返回 drift diff，不直接绕过 review | diff JSON | 本轮未执行 | 待执行 |
| G-TF-001 | Terraform provider build/test | 以 `deploy/terraform/provider/README.md` 为准 | provider build/test 通过 | CI log | 本轮未执行 | 待执行 |
| G-TF-002 | Terraform manifest diff resource | plan/apply 到 dev server | 不绕过 typed CRUD/RBAC/审计 | tf log、audit JSON | 本轮未执行 | 待执行 |
| G-K8S-001 | CRD schema 校验 | kubeconform/kubectl dry-run | CRD schema 合法 | CI log | 本轮未执行 | 待执行 |
| G-K8S-002 | Operator reconcile dry-run | `deploy/smoke/k8s-operator-dry-run-smoke.sh` | status 条件按 manifest diff 更新 | operator log | 本轮未执行 | 待执行 |

## 10. 数据库兼容性专项已验证明细

| 后端 | 版本/资产 | 验证内容 | 当前测试结果 | 状态 |
| --- | --- | --- | --- | --- |
| SQLite | `sqlite::memory:` | schema bootstrap、幂等迁移、repository CRUD smoke | 已通过 | 通过 |
| PostgreSQL | Docker `postgres:16-alpine` | schema bootstrap、幂等迁移、repository CRUD smoke | 已通过 | 通过 |
| MySQL | Docker `mysql:8.4` + `utf8mb4` | schema bootstrap、复合索引长度、text/json 字段、repository CRUD smoke | 已通过 | 通过 |

已修复并验证的问题：

1. MySQL 复合索引字段过长：worker logical identity 复合索引字段改用短字符串列。
2. MySQL `varchar` 容量不足：脚本内容、workflow definition/config/shard payload、audit detail/before/after、schedule calendar JSON 等改为 `text`。
3. 跨库验证资产补齐：`scripts/db-compat-smoke.sh` + `deploy/compose/database-compat-compose.yml` + `crates/tikee-storage/tests/database_compat.rs`。

## 11. 当前总览

| 分类 | 总项数 | 通过 | 待执行 | 失败 | 阻塞 | 跳过 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| P0-A 静态/单元/DB | 17 | 6 | 11 | 0 | 0 | 0 |
| P0-B Server + Java demo | 15 | 0 | 15 | 0 | 0 | 0 |
| P0-C Server + Web | 12 | 0 | 12 | 0 | 0 | 0 |
| P0-D 三端双 worker e2e | 10 | 0 | 10 | 0 | 0 | 0 |
| P1-E SDK Management/API-Key | 7 | 0 | 7 | 0 | 0 | 0 |
| P1-F 脚本沙箱/插件 | 9 | 0 | 9 | 0 | 0 | 0 |
| P2-G GitOps/IaC | 6 | 0 | 6 | 0 | 0 | 0 |
| 数据库专项明细 | 3 | 3 | 0 | 0 | 0 | 0 |

## 12. 下一步执行建议

1. 先执行 P0-A 剩余项：clippy、workspace tests、web tests/build、Java SDK/demo tests。
2. 再执行 `deploy/smoke/java-demo-integration-smoke.sh`，补齐 server + Java demo 核心链路。
3. 再执行 `deploy/smoke/web-live-smoke.sh`，补齐 UI live 验证。
4. 最后执行 `deploy/smoke/server-web-java-joint-e2e.sh`，补齐双 Java worker master/failover 端到端。
5. 每次执行后将本文件对应行的“当前测试结果”和“状态”改为实际结果，禁止未跑即标通过。
