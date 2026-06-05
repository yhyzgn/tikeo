# 148 — Phase 4 Deployment and Browser Evidence Hardening

## 接手上下文

先阅读：`../prompt.md`、`../.memory/next.md`、`../.memory/progress.md`、`../.memory/decisions.md`、`../design/tikee-architecture-design.md`、`../design/server-web-java-joint-automation-test-plan.md`。

当前已完成基线：

- Cross-language Java/Go/Rust Worker parity harness 已纳入主 CI 并上传 artifact。
- GitHub Actions Node runtime policy 已验证无 Node.js 20 warning；Docker validation 已拆分 server/web。
- SQLite legacy schema compatibility 已迁入显式 SeaORM migration `sqlite_compat`，不再通过 `connect_and_migrate` 后置未记录 hook 执行。

## 下一阶段目标

把当前“本地/CI 可跑”的服务能力推进到更接近生产交付的部署和验收证据：

1. **源码行数债务先清理或明确定义豁免**：2026-06-05 audit 发现若干历史文件超过 1500 行，后续不能继续宣称全仓库已满足该规则。
2. **部署硬化**：完善 Helm/外部数据库/TLS/mTLS/Secret/readiness/liveness/worker identity 配置与回滚文档。
3. **浏览器证据硬化**：用 Playwright 或等价真实浏览器测试产出 Workers 分组页、dispatch queue 二级页、API-Key 页截图/视频 artifact。
3. **持续遵守质量门**：不得新增伪实现、未记录 migration、假 capability、命名约定匹配或中英混合 i18n 文案。

## 推荐工作切片

### Slice A — 源码行数债务审计/拆分

- 先运行全仓库行数审计，确认超过 1500 行的 Rust/Web/SDK/demo 源文件。
- 对非生成文件优先按职责拆分；如果文件确为生成产物（例如 API client），必须建立明确生成文件豁免规则和 CI 边界。
- 不要把历史超限状态写成已满足。

### Slice B — 部署配置审计

- 检查 `deploy/`、`config/`、`.github/workflows/`、Dockerfile、compose 文档。
- 输出缺口清单：外部 PostgreSQL/MySQL/CockroachDB、TLS/mTLS secret、Worker identity env、server/worker readiness/liveness、rollback 指南。
- 只记录真实缺口，不要把未实现能力写成完成。

### Slice C — Helm/外部 DB/TLS 最小生产可用切片

- 若 Helm chart 已存在，补 values/schema/templates/tests；若不存在，先创建最小可安装 chart。
- 外部 DB URL、secretRef、TLS cert/key/CA、worker tunnel endpoint 必须结构化配置。
- 增加 template/render/lint 测试；能本地验证的必须本地验证。

### Slice D — Web browser evidence

- 在不破坏现有 Bun/Vite 测试的前提下补 Playwright 真实浏览器 smoke。
- 至少覆盖 `/workers`、`/workers/dispatch-queue`、`/api-keys`。
- 产物必须包含 screenshot/video 或 trace，CI 上传 artifact。

## 必跑验证

根据改动范围选择并记录证据：

```bash
cargo fmt --all -- --check
cargo test -p tikee-storage -- --nocapture
cargo test -p tikee-server -- --nocapture
cd web && bun run typecheck && bun run lint && bun test && bun run build
python3 .github/tests/workflow_contract_test.py
scripts/verify-github-actions-node-runtime.py --min-node-major 24
git diff --check
```

如引入 Helm/Playwright：必须增加对应 lint/test 命令并写入 `.memory/progress.md` / `.memory/session-log.md`。

## 交付规则

- CI job grouping must stay runtime-oriented: Server, Web, Java/Rust/Go SDK+demo, Python/Node.js deferred gates until implemented, and Other for deploy/smoke/Docker/policy.
- 完成后更新 `design/`、`.memory/`、下一编号 `.prompt/`。
- 提交信息遵循 Lore protocol。
- 验证通过后提交并推送；推送后监控 GitHub CI 到完成。
