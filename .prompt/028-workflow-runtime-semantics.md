# 028-workflow-runtime-semantics

## 背景
027 已补齐工作流节点属性编辑：Job 节点可绑定具体任务，Script/HTTP/Condition/Parallel/Join/Delay/Approval/Notification 等节点可配置语义字段，后端 definition validation 已做基础必填校验。

## 下一阶段目标
把当前“可配置/可校验”的节点推进为“可执行/可恢复”的运行时语义：
1. Job：保持创建 job_instance + dispatch_queue。
2. Condition：执行安全表达式，true -> succeeded，false -> failed 或专用 edge label，再推进分支。
3. Parallel：自动推进所有出边分支。
4. Join：明确 all/quorum/any 策略并实现汇聚判断。
5. Delay：按 seconds 写入 dispatch_queue.run_after，延迟后自动推进。
6. Approval：进入 waiting_approval 状态，新增审批 HTTP API，审批通过/拒绝后推进。
7. Notification：接入现有 alert/notification 抽象，发送后标记成功或失败。
8. Script/HTTP：结合动态脚本安全沙箱和 HTTP allowlist/timeout/retry 策略，避免任意网络/命令风险。

## 约束
- API 必须保持 `{ code, message, data }`。
- 数据库禁止外键。
- 禁止 Swagger。
- 每个运行语义必须有测试覆盖：validation、materialize/advance、失败恢复。
