# Server + Web + Java SDK/Demo 联合自动化测试方案

> **执行对象:** tikee server、web 控制台、Java SDK、Java Spring Boot worker demo。  
> **方案目标:** 按功能/测试项清单逐项做“功能预期测试”，每一项必须验证业务结果符合预期，而不只是编译、启动或接口返回 2xx。  
> **状态口径:** `📝 执行时回填` / `执行中` / `通过` / `失败` / `阻塞` / `跳过`。初始状态统一为 `📝 执行时回填`，执行后由测试负责人或 CI 报告回填。

## 1. 总体执行原则

1. **隔离环境优先**：自动化测试默认使用临时 SQLite DB、独立端口和 `.dev/reports/<run-id>/` 报告目录，不污染本地 `tikee-dev.db`。
2. **先窄后宽**：先跑静态/单元测试，再跑 server + Java demo smoke，最后跑 server + web + Java demo 端到端。
3. **所有命令用 `rtk` 包装**：保持与本项目当前联调习惯一致，便于后续在 RTK 环境中复用。
4. **证据必须落盘**：每个自动化阶段至少产出命令日志、JSON/HTML 报告或 API 响应快照。
5. **失败即停**：CI 中任一 P0/P1 测试项失败即终止；P2 可按 nightly 策略收集失败后继续。
6. **功能预期优先**：每个测试项必须同时检查“系统做了正确的事”。只通过编译、启动、HTTP 2xx 或页面能打开，不允许标记为 `通过`。
7. **断言必须可复盘**：测试结果必须落成字段级 API 响应、实例状态、日志内容、数据库快照、截图或视频之一；人工肉眼判断不能作为唯一证据。
8. **禁止约定式能力匹配回退**：worker 能力、插件处理器、脚本沙箱、master election 等必须验证结构化字段，不接受仅靠字符串拼接通过。

## 2. 推荐测试环境

| 项 | 建议值 | 说明 | 状态 |
| --- | --- | --- | --- |
| OS | Linux/macOS 开发机或 CI runner | Java demo、server、web 同机联调 | 📝 执行时回填 |
| Rust | 项目当前 toolchain | 以 `cargo` 实测为准 | 📝 执行时回填 |
| Bun/Node | web 现有依赖要求 | `web/package.json` 中脚本使用 Bun | 📝 执行时回填 |
| Java | JDK 17+ | Spring Boot demo / Gradle | 📝 执行时回填 |
| SQLite | 本地文件 | 自动化使用 `.dev/e2e/*.db` | 📝 执行时回填 |
| curl/python3 | 必须存在 | smoke 脚本依赖 | 📝 执行时回填 |
| 浏览器 | Chromium | 后续 Playwright/截图验收 | 📝 执行时回填 |

## 3. 端口与目录约定

| 组件 | 默认开发端口 | 自动化推荐端口 | 健康检查 | 状态 |
| --- | ---: | ---: | --- | --- |
| Server HTTP | 9090 | 19090 | `GET /readyz` | 📝 执行时回填 |
| Worker Tunnel | 9998 | 19998 | Java worker 注册后查 `/api/v1/workers` | 📝 执行时回填 |
| Web Vite | 5173 | 15173 | `GET /` 或 Playwright 访问 | 📝 执行时回填 |
| Java demo A | 18080 | 18080 | `GET /demo/health` | 📝 执行时回填 |
| Java demo B | - | 18081 | `GET /demo/health` | 📝 执行时回填 |
| 报告目录 | - | `.dev/reports/<run-id>/` | 存放日志、JSON、截图 | 📝 执行时回填 |

建议统一设置：

```bash
export TIKEE_E2E_RUN_ID="joint-$(date -u +%Y%m%dT%H%M%SZ)-$$"
export TIKEE_E2E_REPORT_DIR="$PWD/.dev/reports/$TIKEE_E2E_RUN_ID"
mkdir -p "$TIKEE_E2E_REPORT_DIR"
```

## 4. 一键执行总览

> 当前仓库提供 `deploy/smoke/java-demo-integration-smoke.sh`、`deploy/smoke/web-live-smoke.sh`、`deploy/smoke/server-web-java-joint-e2e.sh` 等脚本，覆盖 server + Java demo、Web live route、双 Java worker master/failover 联合自动化测试。

```bash
# 0) 基础检查
rtk bash -lc 'git status --short -- . ":!.omx"'

# 1) Server 静态/单元验证
rtk cargo fmt --all -- --check
rtk cargo clippy --workspace --all-targets --all-features -- -D warnings
rtk cargo test --workspace --all-features
rtk cargo test -p tikee-server raft -- --nocapture
rtk cargo test -p tikee-server worker -- --nocapture

# 2) Web 静态/单元/构建验证
rtk bun --prefix web test -- --run
rtk bun --prefix web run typecheck
rtk bun --prefix web run lint
rtk bun --prefix web run build

# 3) Java SDK / demo 单元验证
rtk bash -lc 'cd sdks/java && ./gradlew test --no-daemon'
rtk bash -lc 'cd examples/java/spring-worker-demo && ./gradlew test --no-daemon'

# 4) Server + Java demo 集成 smoke
rtk bash deploy/smoke/java-demo-integration-smoke.sh
```

## 5. 功能预期断言规范

所有测试项在回填 `通过` 前必须满足下表的“功能预期断言”。编译/启动/接口可访问只算前置条件，不算最终通过条件。

| 验证层级 | 不能只验证 | 必须验证的功能预期 | 证据类型 | 状态 |
| --- | --- | --- | --- | --- |
| Server | 进程启动、接口 2xx | 数据模型、状态机、调度结果、审计/日志、权限边界符合设计 | API JSON、DB 快照、server log、单测输出 | 📝 执行时回填 |
| Web | 页面能打开、构建成功 | 路由、交互、表单、脱敏、复制、全屏、画布、错误提示与 API 数据一致 | screenshot、DOM assert、network payload、console log | 📝 执行时回填 |
| Java SDK | Gradle test 通过 | SDK 生成正确请求、结构化注册、API-Key 鉴权、worker 上报和错误处理符合协议 | Gradle report、mock server request、live API 响应 | 📝 执行时回填 |
| Java demo | Spring Boot 启动 | processor 被真实分发、业务日志进入实例日志、成功/失败状态正确回写 | demo log、instance JSON、instance logs JSON | 📝 执行时回填 |
| 三端联合 | 单链路成功 | server/web/sdk/demo 对同一业务对象的视图一致，故障切换后仍符合预期 | report JSON、screenshots、timeline logs | 📝 执行时回填 |

通过判定公式：

```text
通过 = 前置条件成功 + 操作步骤完成 + 业务预期命中 + 反向失败条件未出现 + 证据落盘
```

任何一项缺失都只能标为 `失败`、`阻塞` 或 `待补证据`，不能标为 `通过`。

## 6. 功能/测试项清单

### 6.1 P0 阶段 A：静态、编译、单元 + 关键功能预期测试

| ID | 功能/测试项 | 覆盖组件 | 执行命令 | 断言标准 | 证据产物 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A-SRV-001 | Rust 格式检查 | server/storage/proto/sdk rust | `rtk cargo fmt --all -- --check` | exit code = 0 | CI log | 📝 执行时回填 | 合并前必跑 |
| A-SRV-002 | Rust Clippy 全工作区 | server/storage/proto | `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` | 无 warning/error | CI log | 📝 执行时回填 | 不允许 `#[allow(clippy::too_many_lines)]` 掩盖大文件问题 |
| A-SRV-003 | Rust 全工作区测试 | server/storage/proto | `rtk cargo test --workspace --all-features` | 不仅全部通过，还必须确认关键状态机断言覆盖 storage schema、dispatch、auth、workflow、logs | CI log | 📝 执行时回填 | 基础功能预期回归 |
| A-SRV-004 | Server Raft 自主选主测试 | server | `rtk cargo test -p tikee-server raft -- --nocapture` | Raft 相关测试通过 | CI log | 📝 执行时回填 | 覆盖 server 服务集群 master election |
| A-SRV-005 | Worker registry/master 测试 | server worker tunnel | `rtk cargo test -p tikee-server worker -- --nocapture` | Worker registry/master dispatch 测试通过 | CI log | 📝 执行时回填 | 覆盖 worker 服务集群 master election |
| A-WEB-001 | Web Vitest | web | `rtk bun --prefix web test -- --run` | 不仅全部通过，还必须覆盖路由守卫、字段映射、表单 payload、状态展示等业务预期 | CI log | 📝 执行时回填 | 包含路由/页面/API client 回归 |
| A-WEB-002 | Web TypeScript 类型检查 | web | `rtk bun --prefix web run typecheck` | exit code = 0 | CI log | 📝 执行时回填 | 防止 API 类型漂移 |
| A-WEB-003 | Web lint | web | `rtk bun --prefix web run lint` | exit code = 0 | CI log | 📝 执行时回填 | UI 代码质量门禁 |
| A-WEB-004 | Web 生产构建 | web | `rtk bun --prefix web run build` | dist 构建成功 | `web/dist` / CI log | 📝 执行时回填 | 验证刷新路由 404 修复不破坏构建 |
| A-JAVA-001 | Java SDK 单元测试 | Java SDK | `rtk bash -lc 'cd sdks/java && ./gradlew test --no-daemon'` | 不仅全部通过，还必须确认 SDK 请求结构、API-Key header、worker registration/election payload 符合协议 | Gradle test report | 📝 执行时回填 | 包含 management/API-Key/worker client |
| A-JAVA-002 | Java worker client targeted 测试 | Java SDK | `rtk bash -lc 'cd sdks/java && ./gradlew :tikee:test --tests com.yhyzgn.tikee.worker.client.GrpcTikeeWorkerClientTest --no-daemon'` | 全部通过 | Gradle test report | 📝 执行时回填 | 验证结构化 registration/election |
| A-DEMO-001 | Java Spring demo 单元测试 | Java demo | `rtk bash -lc 'cd examples/java/spring-worker-demo && ./gradlew test --no-daemon'` | 全部通过 | Gradle test report | 📝 执行时回填 | demo processor 与配置检查 |

### 6.2 P0 阶段 B：Server + Java demo 集成 smoke

| ID | 功能/测试项 | 覆盖组件 | 执行方式 | 断言标准 | 证据产物 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| B-BOOT-001 | 临时 server 启动 | server | `rtk bash deploy/smoke/java-demo-integration-smoke.sh` 自动启动 | `GET /readyz` 成功 | `.dev/reports/*-server.log` | 📝 执行时回填 | 默认端口 19090/19998 |
| B-AUTH-001 | 初始化管理员注册/登录 | server auth | smoke 先查 `GET /auth/bootstrap`，未初始化时 `POST /auth/bootstrap/register`，已初始化时 `POST /auth/login` | 返回 `data.token`，且注册入口关闭 | smoke JSON / log | 📝 执行时回填 | 用户由 smoke 环境变量指定，默认 `smoke_admin` |
| B-WORKER-001 | Java demo 启动 | Java demo | smoke 自动 `./gradlew bootRun` | `GET /demo/health` 成功 | `.dev/reports/*-java-demo.log` | 📝 执行时回填 | 默认端口 18080 |
| B-WORKER-002 | Worker 注册在线 | server + Java SDK/demo | smoke 查询 `/api/v1/workers` | `spring-demo-worker` online，且 namespace/app/cluster/region、processorNames、pluginProcessors、script runtimes 与 demo 配置一致 | smoke JSON / API 响应 | 📝 执行时回填 | 需验证结构化能力而非只看 online |
| B-WORKER-003 | Worker 结构化 election | server + Java SDK | 扩展 smoke 查询 `/api/v1/workers` | 返回 `master.domain/isMaster/masterWorkerId/term/fencingToken` | workers JSON | 📝 执行时回填 | 不接受字符串约定替代 |
| B-JOB-001 | API single 成功任务 | server + Java processor | smoke 创建并触发 `demo.echo` | instance `succeeded`，assigned worker 与 eligible/master 策略一致，实例日志包含 echo processor 的预期业务输出 | smoke JSON / instance API / logs API | 📝 执行时回填 | 验证 processor_name 路由与业务结果 |
| B-JOB-002 | API single 失败任务 | server + Java processor | smoke 创建并触发 `demo.fail` | instance `failed`，失败 message 与 demo 预期异常一致，且失败日志可查询 | smoke JSON / instance/log API | 📝 执行时回填 | 验证失败状态和错误语义持久化 |
| B-JOB-003 | Broadcast 任务 | server + Java processor | smoke 触发 `demo.context` broadcast | parent/attempt `succeeded` | smoke JSON | 📝 执行时回填 | broadcast 不受 master-only 限制 |
| B-JOB-004 | Fixed-rate 任务 | scheduler + Java processor | smoke 创建 fixed_rate `demo.heartbeat` | 至少 1 个 fixed_rate instance `succeeded` | smoke JSON | 📝 执行时回填 | 验证调度 tick |
| B-JOB-005 | Cron 任务 | scheduler + Java processor | smoke 创建 cron `demo.report` | 至少 1 个 cron instance `succeeded` | smoke JSON | 📝 执行时回填 | 验证 cron tick |
| B-WF-001 | 工作流 job 节点 | workflow + dispatcher + Java | smoke create/run/materialize | workflow instance `succeeded` | smoke JSON | 📝 执行时回填 | 覆盖 materialize-next |
| B-LOG-001 | 实例日志持久化 | server + SDK logs | 查询 `/api/v1/instances/{id}/logs` | 包含 demo 执行日志且无重复爆量 | logs JSON | 📝 执行时回填 | 重点覆盖 stdout/log 上报策略 |

推荐直接执行：

```bash
export TIKEE_INTEGRATION_REPORT_DIR="$PWD/.dev/reports"
rtk bash deploy/smoke/java-demo-integration-smoke.sh
```

执行成功后检查：

```bash
ls -lh .dev/reports/*java-demo*.json .dev/reports/*java-demo*-server.log .dev/reports/*java-demo*-java-demo.log
python3 -m json.tool .dev/reports/*java-demo*.json | sed -n '1,220p'
```

### 6.3 P0 阶段 C：Server + Web 联合验证

| ID | 功能/测试项 | 覆盖组件 | 执行方式 | 断言标准 | 证据产物 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| C-WEB-001 | Web dev server 启动 | web | `rtk bun --prefix web run dev -- --host 127.0.0.1 --port 15173` | `/` 可访问只是前置条件；最终必须无 console error，API base/proxy 指向测试 server，首屏数据请求符合预期 | web log / browser console | 📝 执行时回填 | 建议由脚本后台启动 |
| C-WEB-002 | 根路径默认路由 | web | 浏览器/API 访问 `http://127.0.0.1:15173/` | 跳到总览页面或渲染总览 | screenshot / DOM assert | 📝 执行时回填 | 覆盖“直接访问域名默认总览” |
| C-WEB-003 | 会话有效时访问 login | web + auth | 登录后访问 `/login` | 自动跳过 login，回到总览 | screenshot / URL assert | 📝 执行时回填 | 覆盖登录态路由守卫 |
| C-WEB-004 | 刷新二级路由 | web | 直接刷新 `/api-keys`、`/jobs/:id/topology`、`/workflows/:id/designer` | 不应 404 | screenshot / status | 📝 执行时回填 | 验证 SPA fallback |
| C-WEB-005 | Worker 列表显示 | web + server | Web 打开 Workers 页 | 页面字段与 `/api/v1/workers` 一致：状态、结构化 capabilities、processorNames、pluginProcessors、master/follower、domain 不丢失不误显 | screenshot + workers JSON | 📝 执行时回填 | 数据来自 `/api/v1/workers` |
| C-WEB-006 | API-Key 页面 | web + server | 创建/编辑 API-Key | 创建弹窗 key 可点击复制，列表不泄露明文 | screenshot / API assert | 📝 执行时回填 | 覆盖 SDK API-Key UI |
| C-WEB-007 | 任务拓扑二级页 | web + server | 打开任务拓扑页面 | 画布渲染、全屏切换、箭头避让/动画正常 | screenshot/video | 📝 执行时回填 | 图形回放基础 |
| C-WEB-008 | 工作流画布 | web + server | 打开 workflow designer | 全屏切换、实线流动动画正常 | screenshot/video | 📝 执行时回填 | 与任务拓扑一致交互 |
| C-WEB-009 | 插件处理器任务创建 | web + server | 创建插件类型任务 | 处理器/插件字段来自结构化候选项，不手填错配 | API payload / screenshot | 📝 执行时回填 | 禁止字符串约定 |

当前建议新增脚本：`deploy/smoke/web-live-smoke.sh`。脚本应完成：启动/复用 server、启动 web、登录、访问关键路由、保存截图与控制台错误日志。未新增脚本前，可用手工 Playwright 或浏览器录制执行并回填状态。

### 6.4 P0 阶段 D：Server + Web + 双 Java demo 端到端

| ID | 功能/测试项 | 覆盖组件 | 执行步骤 | 断言标准 | 证据产物 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| D-BOOT-001 | 启动隔离 server | server | 临时 config：HTTP 19090、tunnel 19998、DB `.dev/e2e/<run-id>.db` | `/readyz` 成功 | server log/config | 📝 执行时回填 | 可复用 smoke config 模板 |
| D-BOOT-002 | 启动 Java demo A | Java demo | `TIKEE_DEMO_SERVER_PORT=18080`，同 election domain | `/demo/health` 成功 | demo A log | 📝 执行时回填 | client instance 建议 `spring-demo-worker-a` |
| D-BOOT-003 | 启动 Java demo B | Java demo | `TIKEE_DEMO_SERVER_PORT=18081`，同 election domain | `/demo/health` 成功 | demo B log | 📝 执行时回填 | client instance 建议 `spring-demo-worker-b` |
| D-ELECT-001 | 同 domain 唯一 worker master | server + Java SDK/demo + web | 查询 `/api/v1/workers` 并打开 Workers 页 | 同 domain 仅 1 个 `isMaster=true`，另一个 follower | workers JSON + screenshot | 📝 执行时回填 | 生产级关键验收 |
| D-DISP-001 | Single 任务优先 master | dispatcher + Java demo | 创建/触发 single job `demo.echo` | instance 成功，执行 worker 必须等于触发时该 domain 的 master；若落到 follower 即失败 | instance JSON/logs/workers-before JSON | 📝 执行时回填 | 验证 master-first dispatch 业务预期 |
| D-DISP-002 | Broadcast 任务发给所有 worker | dispatcher + Java demo | 创建/触发 broadcast `demo.context` | 两个 worker 都有 attempt/日志 | instance/logs JSON | 📝 执行时回填 | 不受 master-only 限制 |
| D-FAILOVER-001 | Master demo 停止后 follower 晋升 | worker election | kill 当前 master demo 进程 | 另一个 worker 变 `isMaster=true` | workers JSON timeline | 📝 执行时回填 | 需要轮询至 lease/transport error 生效 |
| D-FAILOVER-002 | failover 后 single 任务成功 | dispatcher + Java demo | 再触发 `demo.echo` | instance 成功，worker 为新 master | instance JSON/logs | 📝 执行时回填 | 验证无额外锁情况下有序调度 |
| D-WEB-001 | Web Worker 页展示切换 | web + server | failover 前后各截图一次 | UI Master/Follower 状态随 API 改变 | screenshots | 📝 执行时回填 | 验收可观测性 |
| D-WEB-002 | Web 实例详情日志 | web + server | 打开实例详情 | 控制台/processor 输出显示在实例日志中且无重复 | screenshot + logs JSON | 📝 执行时回填 | 覆盖前期日志问题 |

建议把此阶段脚本化为：`deploy/smoke/server-web-java-joint-e2e.sh`。脚本输出：

```text
.dev/reports/<run-id>/
  config.toml
  server.log
  web.log
  java-demo-a.log
  java-demo-b.log
  workers-before.json
  workers-after-failover.json
  instances.json
  instance-logs.json
  screenshots/
  report.json
```

### 6.5 P1 阶段 E：SDK Management / API-Key 联合验证

| ID | 功能/测试项 | 覆盖组件 | 执行方式 | 断言标准 | 证据产物 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| E-KEY-001 | 后台创建 SDK API-Key | server + web | Web/API 创建 key | key 格式 `tk-` + 64 位大小写字母数字 | API response screenshot | 📝 执行时回填 | 只在创建弹窗显示明文 |
| E-KEY-002 | 创建时复制提醒 | web | 点击 key 文本 | hover primary、cursor pointer、复制成功提示 | screenshot/video | 📝 执行时回填 | 弹窗必须手动确认关闭 |
| E-KEY-003 | 列表脱敏显示 | web | 打开 API-Key 列表 | 中间脱敏，两端明文，无复制按钮 | screenshot | 📝 执行时回填 | 防止复制脱敏值误用 |
| E-KEY-004 | 编辑名称/作用域/有效期 | server + web | 编辑 API-Key | key 值不变，元数据更新 | API response / audit | 📝 执行时回填 | 不再“刷新生成新 key” |
| E-KEY-005 | Java management client 使用 key | Java SDK + server | Java SDK management 测试 | 可按 app scope 调用允许接口，越权失败 | Gradle report | 📝 执行时回填 | SDK 端不走用户 token |
| E-KEY-006 | 审计记录 | server | 查 audit logs | create/update/revoke/use 有审计 | audit JSON | 📝 执行时回填 | 权限链路闭环 |

### 6.6 P1 阶段 F：脚本沙箱与插件任务联合验证

| ID | 功能/测试项 | 覆盖组件 | 执行方式 | 断言标准 | 证据产物 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F-SCRIPT-001 | Shell 脚本任务 | server + Java SDK/demo | 创建/触发 shell script job | 分发到 worker，沙箱执行成功 | instance/logs | 📝 执行时回填 | 默认 auto sandbox |
| F-SCRIPT-002 | Python 脚本任务 | server + Java SDK/demo | 创建/触发 python script job | 不因 `script:python` 能力字符串缺失而卡 dispatching | instance/logs | 📝 执行时回填 | worker 统一接收，worker 侧选沙箱 |
| F-SCRIPT-003 | JavaScript 脚本任务 | server + Java SDK/demo | 创建/触发 JavaScript job | 自动使用 Deno/V8 类沙箱 | instance/logs | 📝 执行时回填 | 语言全称 JavaScript |
| F-SCRIPT-004 | TypeScript 脚本任务 | server + Java SDK/demo | 创建/触发 TypeScript job | 自动使用 Deno/V8 类沙箱 | instance/logs | 📝 执行时回填 | 语言全称 TypeScript |
| F-SCRIPT-005 | Rhai 脚本输出 | Java SDK/demo | 触发 rhai job | print 输出进入 worker 控制台与实例日志，无重复 | console + logs | 📝 执行时回填 | 覆盖前期 print 不显示问题 |
| F-SANDBOX-001 | wasmtime/wasmedge/deno/rhai/v8/srt 环境检查日志 | Java SDK | 启动 demo | info 日志打印检查、安装、fallback 过程 | demo log | 📝 执行时回填 | sandbox 工具统一包管理 |
| F-PLUGIN-001 | 插件注册 | server + web | 创建 plugin processor/alert channel | 类型与处理器结构化保存 | API response | 📝 执行时回填 | 不靠 `plugin-processor:<type>` 拼接 |
| F-PLUGIN-002 | 插件类型任务创建 | server + web + Java demo | 选择插件处理器创建任务 | 候选项来自 worker/plugin 结构化注册 | payload + screenshot | 📝 执行时回填 | 不能出现未注册处理器如 `mixed.sql` |
| F-PLUGIN-003 | 插件任务执行日志 | server + Java demo | 触发 `billing.sql-sync` 类任务 | processor 输出进入实例日志，控制台策略一致 | logs JSON/demo log | 📝 执行时回填 | 当前可先不强制 stdout 桥接 |

### 6.7 P2 阶段 G：GitOps/IaC、Terraform Provider、K8s CRD Operator 验证

| ID | 功能/测试项 | 覆盖组件 | 执行方式 | 断言标准 | 证据产物 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| G-GITOPS-001 | Manifest 导出 | server | `GET /api/v1/gitops/manifest` | YAML/JSON 可解析，有 checksum | manifest file | 📝 执行时回填 | 作为 IaC 输入 |
| G-GITOPS-002 | Manifest diff | server + web | `POST /api/v1/gitops/diff` | 返回 drift diff | diff JSON | 📝 执行时回填 | review-first |
| G-TF-001 | Terraform provider build/test | deploy/terraform/provider | provider 测试命令 | build/test 通过 | CI log | 📝 执行时回填 | 具体命令以 provider README 为准 |
| G-TF-002 | Terraform manifest diff resource | Terraform + server | plan/apply 到 dev server | 不绕过 typed CRUD/RBAC/审计 | tf log + audit | 📝 执行时回填 | P2 nightly |
| G-K8S-001 | CRD schema 校验 | deploy/k8s/crd | kubeconform/kubectl dry-run | CRD schema 合法 | CI log | 📝 执行时回填 | 无集群时 dry-run |
| G-K8S-002 | Operator reconcile dry-run | deploy/k8s/operator | 本地 operator 测试 | status 条件按 manifest diff 更新 | operator log | 📝 执行时回填 | 后续接 kind e2e |

## 7. 手工 API 功能断言样例

以下样例不只是“接口能调用”，还要校验字段是否符合业务预期。

登录：

```bash
export API_URL=http://127.0.0.1:19090
export AUTH_TOKEN="$(curl -fsS -X POST "$API_URL/api/v1/auth/bootstrap/register" \
  -H 'content-type: application/json' \
  -d '{"username":"smoke_admin","email":"smoke.admin@example.com","password":"Tikee@2026!","confirmPassword":"Tikee@2026!"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["token"])')"
```

查询 worker 与 master election，并断言同一 domain 只有一个 master：

```bash
curl -fsS "$API_URL/api/v1/workers" \
  -H "authorization: Bearer $AUTH_TOKEN" \
  | tee "$TIKEE_E2E_REPORT_DIR/workers.json" \
  | python3 -m json.tool

python3 - "$TIKEE_E2E_REPORT_DIR/workers.json" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1], encoding='utf-8'))
items = payload.get('data', {}).get('items', [])
assert items, 'expected at least one worker'
by_domain = {}
for item in items:
    master = item.get('master') or {}
    domain = master.get('domain') or item.get('worker_domain') or 'unknown'
    by_domain.setdefault(domain, []).append(item)
for domain, workers in by_domain.items():
    masters = [w for w in workers if (w.get('master') or {}).get('isMaster') is True]
    assert len(masters) == 1, f'domain {domain} expected exactly one master, got {len(masters)}'
print('worker election expectation passed')
PY
```

创建并触发 single job：

```bash
JOB_ID="$(curl -fsS -X POST "$API_URL/api/v1/jobs" \
  -H "authorization: Bearer $AUTH_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"namespace":"default","app":"default","name":"joint-demo-echo","schedule_type":"api","processor_name":"demo.echo","enabled":true}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["id"])')"

INST_ID="$(curl -fsS -X POST "$API_URL/api/v1/jobs/$JOB_ID:trigger" \
  -H "authorization: Bearer $AUTH_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"trigger_type":"api","execution_mode":"single"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["id"])')"

curl -fsS "$API_URL/api/v1/instances/$INST_ID" \
  -H "authorization: Bearer $AUTH_TOKEN" \
  | tee "$TIKEE_E2E_REPORT_DIR/instance-$INST_ID.json" \
  | python3 -m json.tool

curl -fsS "$API_URL/api/v1/instances/$INST_ID/logs" \
  -H "authorization: Bearer $AUTH_TOKEN" \
  | tee "$TIKEE_E2E_REPORT_DIR/instance-$INST_ID-logs.json" \
  | python3 -m json.tool

python3 - "$TIKEE_E2E_REPORT_DIR/instance-$INST_ID.json" "$TIKEE_E2E_REPORT_DIR/instance-$INST_ID-logs.json" <<'PY'
import json, sys
inst = json.load(open(sys.argv[1], encoding='utf-8'))
logs = json.load(open(sys.argv[2], encoding='utf-8'))
assert inst.get('data', {}).get('status') == 'succeeded', 'expected instance succeeded'
text = json.dumps(logs, ensure_ascii=False)
assert 'demo.echo' in text or 'echo' in text.lower(), 'expected echo processor log evidence'
print('single job functional expectation passed')
PY
```

## 8. 测试报告模板

每次执行后生成或回填以下清单，建议保存为 `.dev/reports/<run-id>/joint-report.md`：

```markdown
# Joint automation report

Run ID: <run-id>
Date: <YYYY-MM-DD HH:mm:ss TZ>
Git commit: <sha>
Tester/CI: <name or job url>

| ID | 功能/测试项 | 状态 | 证据 | 失败摘要 | 负责人 |
| --- | --- | --- | --- | --- | --- |
| A-SRV-001 | Rust 格式检查 | 通过 | ci-log-url | - | - |
| B-WORKER-003 | Worker 结构化 election | 📝 执行时回填 | - | - | - |
```

状态回填规则：

- `通过`：命令 exit code = 0，且功能预期命中；必须有字段级断言、截图/视频或日志/API 证据。
- `失败`：命令失败、业务断言失败、只验证了启动/2xx 但缺功能证据，均必须标失败并附日志路径。
- `阻塞`：环境缺失、端口冲突、依赖服务不可用，必须附阻塞原因。
- `跳过`：本轮明确不跑，例如 P2 nightly 项，必须附跳过原因。

## 9. CI 分层建议

| Pipeline | 触发时机 | 必跑测试项 | 失败策略 | 状态 |
| --- | --- | --- | --- | --- |
| PR fast | 每次 PR | A-SRV-001/002/003、A-WEB-001/002/003、A-JAVA-001/002、A-DEMO-001 | 失败阻断 | 📝 执行时回填 |
| PR integration | PR 标记 `integration` 或 main merge 前 | 阶段 B 全部 | 失败阻断 | 📝 执行时回填 |
| Nightly e2e | 每晚 | 阶段 C/D/E/F/G | 收集报告，P0/P1 失败告警 | 📝 执行时回填 |
| Release gate | 发版前 | 全部 P0/P1 + 关键 P2 | 失败阻断 | 📝 执行时回填 |

## 10. 故障排查清单

| 现象 | 优先检查 | 命令/证据 | 状态 |
| --- | --- | --- | --- |
| server 启动失败 | SQLite schema / config / 端口 | `tail -n 200 .dev/reports/*server.log` | 📝 执行时回填 |
| Java demo 启动后退出 | Gradle log / worker endpoint / sandbox installer | `tail -n 200 .dev/reports/*java-demo.log` | 📝 执行时回填 |
| worker 不在线 | `/api/v1/workers`、tunnel 端口、client instance id | `curl $API_URL/api/v1/workers` | 📝 执行时回填 |
| instance 一直 pending/dispatching | dispatch_queue、worker eligibility、结构化能力字段 | instance API + server log | 📝 执行时回填 |
| single job 没派给 master | worker master summary、dispatcher candidate order | workers JSON + instance JSON | 📝 执行时回填 |
| broadcast 只到一个 worker | broadcast selector、worker scope/labels | instance attempts/logs | 📝 执行时回填 |
| Web 刷新 404 | Vite/proxy/SPA fallback 配置 | 浏览器 network + web log | 📝 执行时回填 |
| 实例日志重复或缺失 | SDK log 上报、stdout bridge、server log persistence | demo log + instance logs API | 📝 执行时回填 |
| API-Key 明文泄露 | Web 列表、API response、审计日志 | screenshot + response JSON | 📝 执行时回填 |

## 11. 后续脚本化增强项

| ID | 增强项 | 目标文件 | 验收标准 | 状态 |
| --- | --- | --- | --- | --- |
| S-001 | Web live smoke 脚本 | `deploy/smoke/web-live-smoke.sh` | 自动登录、访问关键路由、保存 route evidence | ✅ 已补充 |
| S-002 | 三端联合 e2e 脚本 | `deploy/smoke/server-web-java-joint-e2e.sh` | 自动启动 server/web/2 个 Java demo 并验证 failover | ✅ 已补充 |
| S-003 | JSON report 聚合器 | `deploy/smoke/collect-joint-report.py` | 把命令结果汇总成带状态 Markdown/JSON | ✅ 已补充 |
| S-004 | CI workflow | `.github/workflows/joint-automation.yml` 或现有 CI | 分层执行 PR fast / integration / nightly | 发布集成项；本轮已提供可调用脚本入口 |

## 12. 当前立即可执行的最小闭环

若只想先快速验证 server + web + Java SDK/demo 主链路，按顺序执行；命令完成后按 smoke report/API/logs 回填功能预期结果：

```bash
rtk cargo test -p tikee-server worker -- --nocapture
rtk bun --prefix web test -- --run
rtk bash -lc 'cd sdks/java && ./gradlew test --no-daemon'
rtk bash -lc 'cd examples/java/spring-worker-demo && ./gradlew test --no-daemon'
rtk bash deploy/smoke/java-demo-integration-smoke.sh
```

完成后把对应 `A-*`、`B-*` 清单状态从 `📝 执行时回填` 回填为 `通过` 或 `失败`，并附 `.dev/reports/` 中的证据路径。

## 13. 完全自动化测试前必须补充的测试资产清单

结论：**需要补充**。当前项目已有较多单元测试和 `deploy/smoke/java-demo-integration-smoke.sh`，但还不足以让本方案“无人值守、全链路、按功能预期断言”地跑完。缺口主要不在编译运行，而在：三端编排、浏览器真实交互、双 Java worker failover、功能预期断言落盘、报告聚合。

### 13.1 现有测试资产盘点

| 资产 | 当前覆盖 | 可直接复用程度 | 缺口 | 状态 |
| --- | --- | --- | --- | --- |
| `deploy/smoke/java-demo-integration-smoke.sh` | server + 单 Java demo；API/fixed-rate/cron/broadcast/workflow smoke | 高 | 已补结构化 worker/election 字段断言、实例日志内容断言；双 worker failover/web 联动由 joint e2e 覆盖 | ✅ 已增强 |
| `crates/tikee-server/src/http/tests/*` | server API、auth、scope、workflow、logs 等 | 高 | server 单元/集成层已覆盖；真实 Java worker/web 由 smoke/e2e 覆盖 | ✅ 已覆盖 |
| `crates/tikee-server/src/cluster/raft_rs/tests.rs` | server Raft 自主选主 | 高 | Raft 自主选主测试已覆盖；多进程 chaos 属发布环境增强 | ✅ 已覆盖 |
| `crates/tikee-server/src/tunnel/*` 测试 | worker registry、dispatcher、worker master | 高 | 单元层已覆盖；真实 Java demo 双 worker 由 joint e2e 脚本覆盖 | ✅ 已覆盖 |
| `web/src/**/*.test.ts(x)` | API client、路由、页面源码级回归 | 中 | 已补 Web live smoke 与路由证据；真实浏览器截图/视频属于可选增强 | ✅ 已补 live smoke；浏览器截图为可选增强 |
| `sdks/java/**/src/test` | Java SDK、Spring starter、sandbox、processor registry | 高 | 已补 SDK API-Key live smoke 脚本入口 | ✅ 已补 live smoke 入口 |
| `examples/java/spring-worker-demo/src/test` | demo 启动、处理器、管理 controller | 高 | 已补双 worker 启动脚本与 joint e2e failover 断言 | ✅ 已补脚本 |
| `deploy/tests/iac_artifacts_test.py` | IaC artifact 静态验证 | 中 | Terraform/K8s dry-run 入口已补；真实集群验证属发布环境增强 | ✅ 已覆盖 |

### 13.2 必须补充的 P0 测试脚本/代码

| ID | 必补资产 | 建议路径 | 作用 | 必须验证的功能预期 | 状态 |
| --- | --- | --- | --- | --- | --- |
| ADD-P0-001 | smoke 公共函数库 | `deploy/smoke/lib/tikee-smoke-lib.sh` | 抽出启动 server、登录、轮询、API JSON 断言、清理进程、报告写入 | 后续脚本不再复制粘贴，失败时能统一输出 server/java/web 日志 | ✅ 已补充 |
| ADD-P0-002 | 增强 Java demo smoke | `deploy/smoke/java-demo-integration-smoke.sh` | 在现有 smoke 上增加字段级断言 | worker structured capabilities、pluginProcessors、processorNames、master election 字段、实例日志内容、失败 message | ✅ 已补充 |
| ADD-P0-003 | 三端联合 e2e 脚本 | `deploy/smoke/server-web-java-joint-e2e.sh` | 自动启动 server + web + Java demo A/B，完整跑 D 阶段 | 同 domain 唯一 master、single 落 master、broadcast 到全部、kill master 后 follower 晋升、Web Worker 页数据一致 | ✅ 已补充 |
| ADD-P0-004 | JSON/Markdown 报告聚合器 | `deploy/smoke/collect-joint-report.py` | 把各脚本产物汇总成带状态的报告 | 每个测试项输出 `id/status/evidence/failure`，缺证据不能通过 | ✅ 已补充 |
| ADD-P0-005 | Web live smoke | `deploy/smoke/web-live-smoke.sh` | 自动启动/复用 web，检查 SPA fallback 与关键路由 | `/`、`/login`、`/api-keys`、`/workers` 返回 SPA shell 且非 404；登录态跳过由 RouteAuth 单测覆盖，真实浏览器登录态另由后续 e2e 验证 | ✅ 已补充 |
| ADD-P0-006 | API 字段断言小工具 | `deploy/smoke/assert_tikee_expectations.py` | 对 workers、instances、logs、api-key 等 JSON 做业务断言 | 不只看 2xx；断言 master 唯一、实例状态、日志内容、脱敏/明文策略等 | ✅ 已补充 |

### 13.3 建议补充的 P1/P2 测试代码

| ID | 建议资产 | 建议路径 | 作用 | 必须验证的功能预期 | 优先级 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| ADD-P1-001 | SDK API-Key live smoke | `deploy/smoke/sdk-api-key-live-smoke.sh` | 连接真实 server 并使用 `x-tikee-api-key` 调用管理链路 | API-Key 格式、namespace/app scope、create/update/use 与 server 状态一致 | P1 | ✅ 已自动化 |
| ADD-P1-002 | Java demo 双 worker 启动脚本支持 | `examples/java/spring-worker-demo/scripts/run-demo-worker.sh` | 统一传入 port/clientInstanceId/election priority/stateDir | 同一代码可启动 A/B 两个稳定 worker，互不抢端口/状态目录 | P1 | ✅ 已自动化 |
| ADD-P1-003 | Web live smoke 路由测试 | `deploy/smoke/web-live-smoke.sh` | 用真实 Vite 服务验证关键 SPA 路由 shell 与非 404 | `/`、`/login`、`/api-keys`、`/workers` 可刷新访问；Playwright 截图为可选增强 | P1 | ✅ 已自动化 |
| ADD-P1-004 | Web 静态/路由测试脚本 | `web/package.json` + `deploy/smoke/web-live-smoke.sh` | 复用现有 `bun test/typecheck/lint/build` 与 live smoke | CI 可一键跑 Web 静态测试和 live route smoke；截图/video 为可选增强 | P1 | ✅ 已覆盖 |
| ADD-P1-005 | 脚本沙箱 live smoke | `deploy/smoke/script-sandbox-live-smoke.sh` | 自动创建 Shell/Python/JavaScript/TypeScript/Rhai 脚本定义 | 服务端脚本语言模型与治理字段可被自动断言；真实执行由 joint/script runtime 环境启用后继续跑 | P1 | ✅ 已自动化 |
| ADD-P1-006 | 插件任务 live smoke | `deploy/smoke/plugin-processor-live-smoke.sh` | 自动创建插件和插件任务 | 插件 processor type 与 processorNames 结构化保存，任务只能引用已声明处理器 | P1 | ✅ 已自动化 |
| ADD-P2-001 | Terraform provider smoke | `deploy/smoke/terraform-provider-smoke.sh` | provider build/test 入口 | provider Go 测试与构建可自动跑；真实 plan/apply 属发布环境增强 | P2 | ✅ 已自动化 |
| ADD-P2-002 | K8s operator dry-run smoke | `deploy/smoke/k8s-operator-dry-run-smoke.sh` | operator Go 测试 + CRD 静态校验入口 | CRD 关键字段和 operator 单元测试可自动跑；真实集群 kind/e2e 属发布环境增强 | P2 | ✅ 已自动化 |

### 13.4 需要补充或强化的单元测试点

| ID | 组件 | 建议测试点 | 原因 | 状态 |
| --- | --- | --- | --- | --- |
| UT-SRV-001 | server dispatcher | single dispatch 结果中记录的 worker 必须等于排序后的 master candidate | 支撑 D-DISP-001 自动断言 | ✅ 已覆盖 |
| UT-SRV-002 | server workers API | `/api/v1/workers` 返回 structured capabilities、plugin processors、processor names、master summary 字段稳定 | Web 和 smoke 都依赖字段级断言 | ✅ 已覆盖 |
| UT-SRV-003 | server logs API | processor stdout/script output 不重复、不丢失、sequence 单调 | 支撑实例日志预期测试 | ✅ 已覆盖 |
| UT-SRV-004 | server SDK API-Key | create/update/revoke/use 审计完整，scope 越权失败 | 支撑 SDK management live smoke | ✅ 已覆盖 |
| UT-WEB-001 | web API client | workers structured fields 类型映射完整 | 防止 capabilities 栏再次丢字段 | ✅ 已覆盖 |
| UT-WEB-002 | web route/auth | SPA fallback、登录态 `/login` bypass、根路径总览 | 支撑无人工刷新验收 | ✅ 已覆盖；live 证据由 smoke/e2e 产物补充 |
| UT-WEB-003 | web API-Key page | 创建明文只在弹窗出现、列表脱敏、编辑不换 key | 涉及安全预期，不能只靠人工看 | ✅ 已覆盖 |
| UT-WEB-004 | web topology/workflow canvas | 全屏按钮、节点/边渲染、动画 class/source 数据存在 | 视觉交互需要自动截图补充 | ✅ 已覆盖 |
| UT-JAVA-001 | Java SDK worker registration | registration request 包含 processorNames/pluginProcessors/scriptRunners/election | 支撑 worker 结构化能力验收 | ✅ 已覆盖；字段全量由新增 workers 断言工具校验 |
| UT-JAVA-002 | Java SDK stdout/log capture | processor 输出进入 worker 控制台且上报实例日志策略可控 | 避免日志重复或缺失 | ✅ 已覆盖 |
| UT-JAVA-003 | Java SDK sandbox resolver | auto 模式对 JavaScript/TypeScript/Python/Shell/Rhai 的选择与 fallback 符合设计 | 支撑脚本沙箱 live smoke | ✅ 已覆盖 |
| UT-DEMO-001 | Java demo plugin processor | demo 只暴露实际注册的插件 processor，如 `billing.sql-sync` | 防止 Web/任务候选项出现不存在处理器 | ✅ 已覆盖；live 证据由插件 smoke 补充 |

### 13.5 推荐的补充顺序

| 顺序 | 动作 | 交付物 | 完成后可解锁 | 状态 |
| --- | --- | --- | --- | --- |
| 1 | 抽 smoke 公共库 + JSON 断言工具 | `deploy/smoke/lib/tikee-smoke-lib.sh`、`deploy/smoke/assert_tikee_expectations.py` | 所有脚本统一功能断言 | ✅ 已完成 |
| 2 | 增强现有 Java demo smoke | `deploy/smoke/java-demo-integration-smoke.sh` | B 阶段真正功能预期自动化 | ✅ 已完成 |
| 3 | 增加三端联合 e2e 脚本 | `deploy/smoke/server-web-java-joint-e2e.sh` | D 阶段双 worker master/failover 自动化 | ✅ 已完成 |
| 4 | 增加 Web live smoke | `deploy/smoke/web-live-smoke.sh`，必要时再引入 Playwright | C 阶段真实路由/页面 shell 证据；浏览器截图可后续增强 | ✅ 已完成 |
| 5 | 增加报告聚合器 | `deploy/smoke/collect-joint-report.py` | 一份完整自动化测试报告 | ✅ 已完成 |
| 6 | 补 P1 live smoke | script sandbox / plugin / API-Key management scripts | E/F 阶段闭环 | ✅ 已完成 |


### 13.6 新增测试资产执行命令

| 场景 | 命令 | 预期结果 | 状态 |
| --- | --- | --- | --- |
| 断言工具单元测试 | `rtk python3 -m unittest deploy.tests.smoke_assertions_test` | 断言工具正反例均通过 | ✅ 已验证 |
| Java demo 增强 smoke | `rtk bash deploy/smoke/java-demo-integration-smoke.sh` | 生成包含 `functional_cases` 的 JSON 报告 | 📝 执行时回填 |
| Web live smoke | `rtk bash deploy/smoke/web-live-smoke.sh` | 根路由、login、api-keys、workers 路由返回 SPA shell 且非 404；不宣称已验证登录态重定向 | 📝 执行时回填 |
| 三端联合 e2e | `rtk bash deploy/smoke/server-web-java-joint-e2e.sh` | server/web/双 Java demo 自动启动，master/failover/dispatch/broadcast 功能预期通过 | 📝 执行时回填 |
| 报告聚合 | `rtk python3 deploy/smoke/collect-joint-report.py .dev/reports/<run-id>` | 生成 `joint-report.json` 和 `joint-report.md` | ✅ 已验证基础聚合 |
