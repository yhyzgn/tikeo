# 下一步

## 当前建议阶段

执行 `.prompt/017-script-versioning-and-diff.md`。

## 目标

为脚本管理增加版本历史（`script_versions` 表）和 diff 对比能力。每次 content 或 policy 变更自动产生版本记录，支持任意两版本间的 content diff 和 policy diff。

## 开始前检查

- 先确认 016-dynamic-script-sandbox 已提交并推送。
- 脚本管理 CRUD 后端与 Web UI 已完成，但版本历史和 diff 对比尚未实现。
- 接口继续遵循 `{code,message,data}` 规范。
- 数据库禁止外键，只允许字段软关联。
- 所有脚本更新操作必须支持 diff 对比（用户决策，见 `.memory/decisions.md`）。
