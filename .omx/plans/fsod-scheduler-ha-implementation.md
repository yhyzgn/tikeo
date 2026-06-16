# FSOD Scheduler HA 改造开发计划

状态：Active  
最后更新：2026-06-16（Phase 1.1-1.6 ✅；验证：cargo test -p tikeo-storage；cargo test -p tikeo-server；cargo clippy -p tikeo-server --all-targets -- -D warnings）  
设计依据：`design/non-lock-distribution-high-performance-scheduler-platform.md`

## 目标

按 FSOD（Fenced Slot Outbox Dispatch）方案，把当前 “Raft 单 Leader + Worker Gateway Relay” 渐进改造成具备 durable dispatch outbox、gateway reroute、visibility timeout、未来 shard ownership 扩展点的生产级闭环。

## 执行原则

- TDD：生产代码前先写失败测试。
- 小步提交：每个可验证阶段通过后再提交。
- 进度清单实时维护：完成项标记为 ✅，部分完成标记为 🚧。
- 正确性优先：Redis/Etcd/NATS 只能作为可选加速，不作为正确性依赖。
- 当前可运行能力不能回退：方案 B 的 Worker gateway relay、assignment token fencing、Web/API node-id 语义必须继续保持。

## 总体阶段清单

### Phase 1：Durable Dispatch Outbox

- [x] 1.1 ✅ 新增 `worker_dispatch_outbox` entity、migration、SQLite compatibility。
- [x] 1.2 ✅ 新增 `WorkerDispatchOutboxRepository`：create/list/claim/mark delivered/mark failed/complete。
- [x] 1.3 ✅ dispatcher 在派发前创建 outbox，且 attempt token 与 outbox token 持久化顺序可测试。
- [x] 1.4 ✅ 新增 gateway delivery loop：扫描本节点 outbox 并投递本地 stream。
- [ ] 1.5 当前 internal relay 降级为 wake-up/hint 路径：relay 失败不丢 dispatch intent。
- [x] 1.6 ✅ 增加 outbox 指标与基础 metrics summary 输出。

验收：gateway/relay 短暂失败后，outbox 保留 queued 状态，恢复后可以投递；token 先落库后投递。

### Phase 2：Outbox Reroute 与 Visibility Timeout

- [ ] 2.1 Worker 重连后根据 `logical_instance_id` + `generation` reroute outbox。
- [ ] 2.2 delivered 未 ack/result 超时后重置为 queued。
- [ ] 2.3 Worker ack/log/checkpoint/result 将 outbox 推进为 acked/completed。
- [ ] 2.4 duplicate dispatch/result 幂等验证。

验收：Worker 在 dispatch 前后重连，任务最终只产生一个 terminal result，无永久 delivered 卡死。

### Phase 3：Raft Shard Ownership 基础

- [ ] 3.1 新增 `cluster_shard_ownership` entity、migration、repository。
- [ ] 3.2 增加稳定 shard key/hash 与 shard map version。
- [ ] 3.3 dispatch_queue 绑定 `shard_id / owner_epoch / owner_fencing_token`。
- [ ] 3.4 owner 只 claim 自己 shard；旧 epoch/fencing 更新被拒绝。
- [ ] 3.5 cluster diagnostics 暴露 shard ownership 摘要。

验收：多 Pod 可分别拥有不同 shard；kill owner 后新 owner 接管，旧 owner 写入失败。

### Phase 4：Locality-Aware Scoring

- [ ] 4.1 实现 LASSO worker scoring。
- [ ] 4.2 本地 gateway worker 优先但不绕过 outbox。
- [ ] 4.3 增加 quota/fairness 指标与测试。

验收：本地可用 Worker 优先，跨 Pod 派发减少，无 worker starvation。

### Phase 5：文档、迁移和运维闭环

- [ ] 5.1 README / docs 更新 FSOD 阶段、配置项、运维限制。
- [ ] 5.2 DB migration 兼容旧库和 SQLite dev DB。
- [ ] 5.3 增加排障 runbook 与 metrics 说明。
- [ ] 5.4 K8s/e2e failover 脚本覆盖 outbox/reroute。

验收：用户仅看 README/docs 即可理解部署模式、限制、故障恢复和验证步骤。

## 当前推进切片

Phase 1 最小生产闭环已完成，下一步进入 Phase 2 reroute/visibility timeout：

1. 红：仓储测试证明 outbox 表不存在/仓储行为缺失。
2. 绿：新增 migration/entity/repository，让 create/list/claim 状态转换通过。
3. 红：dispatcher 测试证明 relay 失败时 outbox queued 保留且 instance 不错误丢失。
4. 绿：dispatcher 写 outbox，并引入 gateway delivery service 的最小路径。
5. 验证：`cargo test -p tikeo-storage`、`cargo test -p tikeo-server tunnel::dispatcher`、`cargo clippy -p tikeo-server --all-targets -- -D warnings`。

## 风险与控制

- Outbox 与旧 relay 双路径可能重复派发：先让 outbox 成为 truth，relay 只做 hint；terminal result 仍由 assignment token fencing 控制。
- 事务边界复杂：attempt token + outbox create + queue running 要么同事务，要么有补偿 scanner。
- DB 写放大：Phase 1 先保证正确性，Phase 4/压测再优化 batch。
