# 017-script-versioning-and-diff

## 背景

016-dynamic-script-sandbox 已完成脚本管理基础切片：
- 后端具备脚本定义 CRUD API（`/api/v1/scripts`），Admin 权限保护。
- Web 管理端已有脚本管理页面（列表、创建、审批、启用/禁用、删除）。
- **缺失**：脚本更新直接覆盖当前记录，没有版本历史和 diff 对比能力。
- 用户要求：所有脚本管理操作必须支持 diff 对比。

## 目标

为脚本管理增加版本历史（`script_versions` 表）和 diff 对比能力。每次 content 或 policy 变更自动产生版本记录，支持任意两版本间的 content diff 和 policy diff。

## 关键约束

- 每次脚本 content 或 policy 字段变更时，必须先将变更前的快照写入 `script_versions`，再更新 `scripts` 主表。
- 版本记录不可修改、不可删除（只增）。
- diff API 支持对比任意两个版本：content 使用 unified diff 格式，policy 使用字段级对比。
- Rust workspace 保持 `crates/*` 模块解耦；根主程序入口仍为 `src/main.rs`。
- Web 保持 `web/` + React + Ant Design + Bun。
- 禁止 Swagger UI，仅保留 `/api-docs/openapi.json`。
- 数据库禁止外键，只允许字段软关联。
- 所有 HTTP 接口继续返回 `{code,message,data}` envelope。

## 建议范围

1. Storage：新增 `script_versions` 表（id/script_id/version/content/language/status/timeout_seconds/max_memory_bytes/allow_network/allowed_env_vars/created_by/created_at），无外键。`script_id` 软关联 `scripts.id`。
2. Repository：`ScriptVersionRepository` 新增 `create_version()`、`list_versions(script_id)`、`get_version(id)`。
3. 修改 `ScriptRepository::update_script()`：更新前自动将当前行快照写入 `script_versions`。
4. HTTP API：
   - `GET /api/v1/scripts/{id}/versions` — 版本历史列表（Admin 权限）。
   - `GET /api/v1/scripts/{id}/diff?v1=N&v2=M` — 两个版本 diff（Admin 权限）。返回 content unified diff + policy 字段对比。
5. OpenAPI：补充版本和 diff 路径与 schema。
6. Web：
   - ScriptsPage 行操作增加"版本历史"按钮，打开版本列表 Drawer。
   - 版本列表 Drawer 中选择两个版本触发 diff 对比视图。
   - diff 视图展示 content unified diff（语法高亮）和 policy 字段变更。
   - 脚本编辑器支持语法检查（Shell/Python/Node 等基础语法校验，实时标红提示）。
7. 脚本编辑器语法检查：
   - Web 脚本创建/编辑 Modal 内嵌语法检查，根据 `language` 字段切换检查器。
   - Shell：基础语法校验（if/fi、case/esac、do/done 配对、未闭合引号等）。
   - Python：基础缩进和语法错误检测。
   - Node/JavaScript：基础语法校验。
   - 可使用 CodeMirror/Monaco Editor 的语言模式，或接入轻量在线 linter。
   - 语法错误实时标红，不阻止保存但必须提示。
8. 测试：版本自动创建、版本列表查询、diff 输出验证、语法检查提示。

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

完成后更新 `design/scheduler-architecture-design.md`、`.memory/*`、后续 `.prompt/018-*.md`，提交并推送。
