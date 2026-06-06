# 024-phase2-distributed-worker-and-recovery

## 背景

023 已补齐 workflow executor 最小推进器、Map/MapReduce/子工作流定义约束、dry-run/advance API、Web DAG 可视化与 SSE 事件流基础。

## 目标

继续 Phase2 分布式执行纵切，让 workflow 节点不只停留在设计/手动推进，而是和 worker dispatch、job instance、map shard、子工作流实例形成可运行闭环。

## 工作项

1. job workflow node：queued node 被 worker/调度器消费时创建或绑定 job_instance，并把 job_instance 状态回写到 workflow_node_instance。
2. map/map_reduce：根据 `map_items` 生成 shard/subtask 最小模型，记录 shard 状态；reduce 在所有 map shard 成功后入队。
3. sub_workflow：根据 `child_workflow_id` 触发 child workflow instance，并把 child 终态映射回父节点。
4. 恢复语义：增加节点重试、跳过、失败恢复 API 的最小设计和实现。
5. Web：补 Worker 集群/队列状态页面基础，展示 worker tunnel、capabilities、pending queue。
6. 文档/记忆：更新 `design/tikeo-architecture-design.md` Phase2 路线图、`.memory/*`，并根据实际进展调整后续 `.prompt`。

## 验证

必须运行并通过：

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

通过后按 Lore Commit Protocol 提交并推送。
