# 029 — 工作流列表运行视图内联化后续

## 已完成上下文
- `/workflows` 一级页面是列表优先，不默认展示画布编辑器。
- 新增/编辑工作流走 `/workflows/new` 与 `/workflows/:id/edit`。
- 本阶段已移除 WorkflowsPage 中全局 AntD Collapse 运行视图列表，不再生成 `运行视图 · <name>` 这类额外 header。
- 点击某条 workflow item 的“运行视图”按钮，会直接在该 item 下方内联展示运行视图和实例事件流；再次点击收起。
- 只允许一个 item 展开，切换 item 时清理旧 activeInstance/events/shards，避免状态误显示。

## 约束
- UI 保持简约浅色现代风格，不要增加厚重面板或全局二级列表。
- 运行态细节应跟随对应列表条目展示，避免重复 header/占位说明。
- API 返回仍必须遵守 `{ code, message, data }`。
- 禁止 Swagger；数据库禁止外键。

## 下一步建议
1. 若继续优化工作流运行体验，优先补齐后端 runtime 语义：condition/parallel/join/delay/approval/notification 的自动推进策略。
2. 若继续优化 UI，建议为运行视图添加 compact 模式和空状态插画，但不要恢复全局 Collapse。
3. 所有变更后执行：`cargo fmt --all`、`bun run --cwd web lint`、`bun run --cwd web typecheck`、`bun test --cwd web`、`bun run --cwd web build`、`cargo clippy --workspace --all-targets --all-features -- -D warnings`、`cargo test --workspace --all-features`、`cargo build --workspace --all-features`、`timeout 10 ./scripts/dev.sh`。

## 追加：二级页返回入口
- Workflow 新增/编辑页顶部 hero 区域已有“← 返回工作流列表”按钮。
- 页面级返回入口保持在 hero 顶部，画布 Card extra 只放预览切换和 Dry-run 等工具动作。

## 追加：运行视图只读约束
- `/workflows` 列表内联运行视图必须只读：禁止节点拖拽、端口连线、边条件编辑、边删除、端点重连。
- 只有 `/workflows/new` 与 `/workflows/:id/edit` 的编辑页可以传入 `DagPreview editable`。
