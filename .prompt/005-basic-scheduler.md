# 005-basic-scheduler：基础调度器与 API 触发链路

## 阶段目标

在已落地的 SeaORM 存储层、SQLite dev DB、Jobs API 和 Worker Tunnel skeleton 基础上，实现最小可用调度核心，为 CRON / Fixed Rate / API 手动触发进入实例生命周期做准备。

## 当前上下文

- 根 `src/main.rs` 是后端 binary entrypoint。
- Rust 模块必须继续放在 `./crates/*`。
- `crates/scheduler-storage` 已提供 namespace/app/job/job_instance schema、migration 与 `JobRepository`。
- `GET /api/v1/jobs` 与 `POST /api/v1/jobs` 已接入真实存储。
- HTTP 业务响应必须保持 `{code,message,data}`，`code=0` 表示成功，`data` 必须显式存在。
- Worker Tunnel gRPC 已有注册/心跳 skeleton，但尚未分发任务。

## 建议任务

1. 在 `scheduler-core` 或新增独立 crate 中定义调度领域模型：ScheduleType、TriggerType、InstanceStatus、DispatchDecision。
2. 扩展 `scheduler-storage`：补充 job_instance repository，支持创建 API 触发实例、查询实例列表/详情。
3. 扩展 HTTP API：
   - `POST /api/v1/jobs/{job}:trigger` 创建手动触发实例。
   - `GET /api/v1/instances` 或 `GET /api/v1/jobs/{job}/instances` 查询实例。
4. 实现最小 API trigger flow：请求 → storage 创建 instance → 返回统一 envelope。
5. 设计但不急于完成 CRON / Fixed Rate tick loop；如完成则必须有确定性测试。
6. 保持 Worker Tunnel 现有注册/心跳测试通过，不要在本阶段强行实现完整远程派发。
7. 更新 OpenAPI schemas 和设计路线图。

## 验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config examples/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/api/v1/jobs
curl -fsS -H 'content-type: application/json' -d '{"namespace":"default","app":"demo","name":"manual-demo"}' http://127.0.0.1:9090/api/v1/jobs
```

完成后更新 `.memory/*`、后续 `.prompt`，提交并推送。
