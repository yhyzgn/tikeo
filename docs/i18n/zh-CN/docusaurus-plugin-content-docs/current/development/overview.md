---
title: 开发与扩展指南
description: 安全开发 Tikeo 的工作流：仓库结构、构建测试、API 变更、Web UI、SDK、文档和扩展边界。
---

# 开发与扩展指南

这页面向 Tikeo 维护者和扩展团队。它解释仓库结构、如何在不破坏运行时契约的情况下改代码、每一层用什么测试证明，以及新行为应该写到哪里。它不是贡献口号，而是实际操作清单：你改一个子系统，就应该知道相邻文件、测试、文档和发布检查如何一起移动。

## 仓库地图

| 路径 | 放什么 | 注意事项 |
| --- | --- | --- |
| `crates/tikeo-config` | 配置结构、默认值、环境变量覆盖行为 | 默认值变化必须更新文档。 |
| `crates/tikeo-server` | HTTP routes、Worker Tunnel、调度、通知、auth、CLI | 运行行为必须真实，不允许占位分支。 |
| `crates/tikeo-storage` | Entity、migration、repository | 数据库敏感改动要加兼容测试。 |
| `crates/tikeo-proto` | Worker Tunnel protobuf 和生成绑定 | 协议变更要同步 SDK 和协议参考。 |
| `web/` | React/TypeScript/Bun 运维控制台 | 用 Bun，UI 文案要过 i18n 测试。 |
| `docs/` | Docusaurus 文档站与 docs Docker 镜像 | 文档必须给人看，且和代码一致。 |
| `sdks/` | Rust/Go/Java/Python/Node SDK | Worker 与 Management 契约要对齐。 |
| `examples/` | 可运行 Worker demo | Demo 应能作为 smoke 目标。 |
| `deploy/` | Compose、Helm、K8s、Terraform、systemd、smoke | 部署文档与测试要同步。 |

## 本地开发循环

用最小循环证明变更：

```bash
cargo fmt --all -- --check
cargo test -p tikeo-server <test-name> --all-features -- --nocapture
bun test web/src/pages/__tests__/NotificationCenterPage.test.tsx
```

较大改动完成前运行：

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
bun test web/src
bun run --cwd web typecheck
bun run --cwd web lint
bun run --cwd web build
python3 scripts/check-source-size.py
git diff --check
```

文档改动还要跑：

```bash
python3 .github/tests/docs_site_contract_test.py
cd docs && bun run docs:typecheck && bun run docs:build
```

## API 变更流程

新增或修改 API route 时：

1. 更新 `crates/tikeo-server/src/http/routes/*` 中的 DTO 和 handler。
2. 需要时更新 `crates/tikeo-server/src/http/router.rs`。
3. 更新 `crates/tikeo-server/src/http/openapi.rs`。
4. 增加成功、认证失败、校验失败、租户/范围保护测试。
5. Web 要使用时更新 `web/src/api`。
6. 更新 reference 和 user guide。
7. 只有当 contract token 能保护真实回归时才添加。

不要在 route 和测试存在之前先写文档声称 API 已实现。

## Web UI 变更流程

Web 使用 React、TypeScript、Vite、Ant Design 和 Bun。规则：

- 前端命令使用 `bun`/`bunx`。
- UI 文案可翻译；更新 zh-CN/en-US 字典。
- 文件变大时按职责拆模块。
- 重要 UX 契约要有源码级回归测试。
- 有意义 UI 改动运行 `bun test web/src`、`bun run --cwd web typecheck`、`bun run --cwd web lint`、`bun run --cwd web build`。

通知中心 UI 要保持边界：channel/template/policy/message/delivery attempt 是 Notification Center 概念；alert firing/recovery/silence 是 Alerts 概念。

## SDK 变更流程

Worker Tunnel 或 Management API 行为变化时：

1. 先改 Rust 源码和协议。
2. 更新每个 SDK helper，或明确记录某个 SDK 的限制。
3. 更新 examples，至少有一个 demo 能证明路径。
4. 运行语言级测试。
5. 更新 SDK 文档：依赖坐标、配置默认值、最小 Worker、Management create/trigger、现场验收。

跨语言 helper 名称必须在文档中可查：`ManagementClient`、`NewManagementClient`、`HttpTikeoJobClient`、`apiJob`、`apiTrigger`、`broadcastApiTrigger`、`BroadcastSelectorRequest`。

## 新增通知 provider

一个 provider 完成前必须具备：

- Provider metadata 和 template schema。
- Config/secret 校验。
- 敏感 target 数据脱敏规则。
- Delivery renderer/sender 或插件边界。
- 如果可安全测试，则实现 test-send。
- Web 抽屉 schema label 和 help text。
- 文档表格和排障说明。

API summary 绝不能返回原始 webhook URL、routing key、signing key、SMTP password、authorization header 或 token-like 值。

## 新增脚本或沙箱能力

脚本能力必须 fail closed。声明能力前：

- 解析 runtime tool path。
- 校验输入和 policy limit。
- 捕获 stdout/stderr/task logs。
- 对 unsupported tool 返回清晰错误。
- 不要在 Worker 真能执行前声称支持 Docker/Podman/WASM/Deno/SRT。

## 文档工作流

文档是产品面。每个功能都要：

- 更新用户路径，而不只是 reference 页。
- 包含前置条件、命令、预期观察、故障排查、生产检查清单。
- 公共文档避免内部交接语言。
- 需要 API 细节时链接精确 reference。
- 优先文档同步 zh-CN 镜像。
- 运行 docs contract 和 Docusaurus build。

## 发布准备

打 tag 前：

- Workspace version 和 lockfile version 必须匹配 tag。
- Server binary `--version` 必须输出 release version。
- Server、Web、Docs Docker images 应用同一 tag 构建发布。
- SDK release workflows 使用同一版本契约。
- GitHub Actions 的 CI、coverage、release assets、Docker images、SDK publish jobs 通过。

## 前置条件

- Rust、Bun、Docker，以及变更涉及的语言 SDK 工具链。
- 本地数据库或隔离 smoke 环境。
- 理解 Server/Worker 边界。
- 大范围重构前先有文档和测试计划。

## 验收

按变更面选择验证：

| 变更面 | 最低验证 |
| --- | --- |
| Rust server/storage | `cargo fmt`、定向测试；广泛改动加 clippy/workspace tests |
| Web | 定向 `bun test`、typecheck、lint、build |
| Docs | docs contract、docs typecheck、docs build |
| SDK | 语言测试；API 行为变更加 management trigger smoke |
| Deploy | Compose/Helm render 和相关 smoke |

## 故障排查

| 问题 | 处理 |
| --- | --- |
| 测试过但 docs fail | 公共契约变了；更新文档或修正已过时测试。 |
| Web i18n fail | 给可见文本、placeholder、label、aria label 加翻译。 |
| source-size fail | 按职责拆文件，不要提高限制。 |
| SDK helper 漂移 | 对比多语言 helper 名称和 examples，再同步文档/测试。 |

## 生产检查清单

- [ ] 真实行为已实现并测试；没有把占位行为说成完成。
- [ ] Runtime、Web、SDK、Docs、Deployment 表面已对齐。
- [ ] 公共文档解释了人如何部署、集成、验收和排障。
- [ ] Release/version 变更同步到 binary、lockfile、images 和 workflows。
