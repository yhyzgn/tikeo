# Next Work

## Current priority direction

当前优先级：把 2026-06-04 已完成的 Java/Go/Rust Worker parity、Worker session snapshot 持久化和 Web Worker 分组验收，从手动联调升级为可重复执行的 cross-language integration harness。最新阶段提示词：`.prompt/147-phase4-cross-language-worker-parity-and-persistence-hardening.md`。

## Immediate next slice — cross-language Worker parity automation

1. 编写 executable harness，启动/验证 Java Boot2、Java Boot3、Java Boot4、Go、Rust demo worker。
2. 使用结构化 namespace/app/cluster/region/clientInstanceId/worker_pool/processorName/processorType 数据，不允许靠命名约定匹配。
3. 触发每个语言 family 的 processor job，特别断言 Go/Rust 实例日志包含 assignment-token 保护下的 received/completed 记录。
4. 增加 server restart persistence smoke：worker 注册后重启 server，验证 `/api/v1/workers` 先从 `worker_sessions` snapshot 展示，再由 live registry 覆盖。
5. 增加 live 与 persisted worker_pool scope filtering 一致性回归。
6. Web smoke 覆盖 Workers 页面 namespace/app -> cluster/region -> node 分组与 `/workers/dispatch-queue` 二级页。
7. 证据统一落盘 `.dev/reports/cross-language-workers-<run-id>/`，并回写 design/docs 与 `.memory`。

## Recent baseline

- Java demos：`examples/java/spring-boot2-worker-demo`、`spring-boot3-worker-demo`、`spring-boot4-worker-demo`。
- Go SDK/demo：`sdks/go/tikee`、`examples/go/worker-demo`，默认 live，README 已说明 protoc/Dockerfile 安装。
- Rust SDK/demo：`sdks/rust/tikee`、`examples/rust/worker-demo`，默认 live。
- Worker visibility：`worker_sessions` 持久化 capabilities/structuredCapabilities/labels/master snapshot；`/api/v1/workers` 合并 live registry 与 DB online sessions。
- Web Worker：按 namespace/app 与 cluster/region 分组；dispatch queue 在 `/workers/dispatch-queue`。
- CI：GitHub Actions run `26947829951` success。

## Standing constraints

- Worker 重要可见性状态不得只在内存。
- 禁止约定命名匹配；必须使用结构化字段、labels 或 structuredCapabilities。
- 中文 i18n 必须完整中文，英文 i18n 必须英文，不要中英混合 label。
- Go/Rust SDK 和 demo 尽量与 Java 一比一对齐；无法对齐时必须写明真实差异。
- 源文件 <=1500 行；`mod.rs` / `lib.rs` 等入口文件只做声明和 re-export。
