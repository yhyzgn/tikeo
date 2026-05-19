# 决策记录

## 2026-05-19 — 开发交接协议

Decision:
- 使用 `prompt.md` 作为跨 AI 智能体开发总提示词。
- 使用 `.memory/` 保存长期项目记忆。
- 使用 `.prompt/` 保存有序阶段提示词。

Rationale:
- 项目周期长，可能由 Codex、Claude、Gemini、OpenCode 等不同智能体接手。
- 需要保证上下文、验证证据、下一步任务和提交状态持续可追溯。

Constraint:
- 每次推进后必须更新记忆库和后续阶段 prompt。


## 2026-05-19 — Rust workspace 与 Web 技术栈约束

Decision:
- Rust 项目必须使用 Cargo workspace。
- 所有 Rust 模块抽取为独立 crate，统一放在 `./crates/` 下。
- Web 管理端必须独立放在 `./web/` 下。
- Web 技术栈固定为 React + TypeScript + Ant Design。
- Web 包管理工具固定使用 Bun。
- 每段代码改动后必须编译、运行、测试，通过后自动提交并推送远程 git。

Rationale:
- 保持后端模块完全解耦，便于长期演进和多智能体并行开发。
- Web 端独立工程能降低前后端耦合，并明确 UI 技术栈。

Constraint:
- 后续不得新建 `webui/` 作为前端工程目录；设计文档中的旧 `webui` 表述应统一迁移为 `web`。


## 2026-05-19 — 依赖版本策略

Decision:
- Rust crate、Web 依赖、构建工具、测试工具和运行时依赖默认使用当前最新稳定版。
- 若因为兼容性、生态稳定性、许可证、安全策略或框架约束不能使用最新版，必须记录原因、锁定版本、风险和未来升级条件。

Rationale:
- 新项目无历史包袱，优先使用最新稳定依赖可以降低后续迁移成本并获得安全修复。

Constraint:
- 禁止随意引入维护停滞、漏洞未修复或生态风险高的依赖。
