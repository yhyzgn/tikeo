# 026-workflow-canvas-editor

## 背景

当前 Workflows 页面已支持轻量可视化编辑：节点拖拽排序、快速新增节点、边关系编辑、JSON 同步。但它仍不是完整画布编辑器。

## 目标

引入或自研真正 DAG canvas：节点坐标、连线拖拽、缩放/平移、节点属性侧栏、局部校验和保存/更新工作流。

## 工作项

1. 评估是否引入 React Flow / X6 等画布库；若新增依赖需确认包体积和维护风险。
2. 设计 WorkflowDefinition 与 canvas node/edge state 的双向转换。
3. 支持拖拽连线、节点坐标、属性面板、删除、复制、自动布局。
4. 支持 workflow update API（当前主要是 create/list/run），并在 Web 中保存已有 workflow。
5. 增加组件级测试和 e2e smoke。

## 验证

至少运行 bun lint/typecheck/test/build；如改后端同步运行 cargo fmt/clippy/test/build。
