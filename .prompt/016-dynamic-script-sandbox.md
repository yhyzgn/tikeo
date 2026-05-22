# 016-dynamic-script-sandbox

## 背景

015-user-management-and-rbac 已完成账号体系接续开发：
- 后端具备 Users CRUD、Admin RBAC、真实 BCrypt 登录。
- Session 管理已抽象为 `SessionStore`，当前实现为 DB `auth_sessions` + moka 本地短缓存，后续可替换/新增 Redis 分布式实现。
- Web 管理端已有用户管理入口。
- 业务 HTTP API 必须继续返回 `{code,message,data}` envelope，`data` 必须显式存在。

## 目标

进入多语言动态脚本与安全沙箱阶段，设计并实现最小可验证切片：脚本定义只由 Server 管理，脚本执行必须发生在 Worker 侧受控沙箱中，Server 不执行用户代码。

## 关键约束

- 支持多语言方向：Shell、Python、Node、PowerShell、Rhai/WASM 等分阶段落地；本阶段可先选一个最小安全执行器，但接口和模型必须可扩展。
- 安全优先：脚本签名/版本、能力声明、资源限制、超时、环境变量白名单、文件系统/网络策略必须进入设计与代码边界。
- Worker 仍只主动连接 Server；Server 通过 Worker Tunnel 下发任务，不直连 Worker。
- Rust workspace 保持 `crates/*` 模块解耦；根主程序入口仍为 `src/main.rs`。
- Web 保持 `web/` + React + Ant Design + Bun。
- 禁止 Swagger UI，仅保留 `/api-docs/openapi.json`。

## 建议范围

1. Storage：新增脚本定义/版本/审批状态的最小表结构。
2. Core：抽象 `ScriptRuntime` / `SandboxPolicy` / `ScriptLanguage` 类型。
3. Worker SDK：预留脚本任务处理入口，至少实现一个安全受限的 mock 或最小 runtime。
4. HTTP API：脚本定义 CRUD、版本查询，Admin 权限保护。
5. Web：脚本管理页面骨架，展示语言、版本、启用状态和安全策略摘要。
6. 测试：模型解析、API envelope、权限拦截、安全策略默认拒绝。

## 验证要求

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
./sdks/java/gradlew -p sdks/java test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
```

完成后更新 `design/tikee-architecture-design.md`、`.memory/*`、后续 `.prompt/017-*.md`，提交并推送。
