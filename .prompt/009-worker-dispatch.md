# 009-worker-dispatch：Worker Tunnel 真实任务分发与执行回传

## 阶段目标

在 API 手动触发实例、Worker 注册/心跳、Rust SDK 处理器 trait 和容器化部署基础已完成后，打通最小真实执行链路：Job API 触发生成 pending instance，Server 通过 Worker Tunnel 将任务下发给在线 Worker，Worker SDK 调用 `TaskProcessor`，再把执行结果回传并更新实例状态。

## 当前上下文

- 根 binary 入口在 `src/main.rs`，业务模块在 `crates/*`。
- HTTP 业务接口必须统一返回 `{ code, message, data }`，`code=0` 表示成功，`data` 必须显式存在。
- Worker Tunnel proto 位于 `proto/scheduler/worker/v1/worker.proto`，RPC 名称为 `OpenTunnel`。
- Server 当前已支持 Worker 注册和 heartbeat，并维护内存 `WorkerRegistry`。
- Storage 当前已支持 `job` 与 `job_instance` 持久化；API 手动触发会创建 pending instance。
- Rust SDK 当前支持主动连接、注册、heartbeat，并定义了 `TaskProcessor` / `TaskContext` / `TaskOutcome` 占位。
- Docker/Compose/K8s baseline 已完成；Worker 必须主动出站连接 Worker Tunnel，不暴露入站端口。

## 建议任务

1. 扩展 Worker Tunnel protobuf：
   - Server -> Worker：dispatch task / cancel task / ping（先实现 dispatch）。
   - Worker -> Server：register / heartbeat / task accepted / task finished。
   - 保持向后清晰命名，避免 tonic client 方法名冲突。
2. Server 侧实现最小 dispatch loop：
   - 查询 pending `job_instance`。
   - 选择在线 worker（先使用简单 first-available / capability placeholder）。
   - 通过该 worker 的 tunnel 下发任务。
   - 更新实例状态为 running / succeeded / failed。
3. Rust Worker SDK：
   - 接收 dispatch message。
   - 构造 `TaskContext`。
   - 调用注册的 `TaskProcessor`。
   - 回传 `TaskOutcome`。
4. Storage/API：
   - 必要时补充 instance 状态更新 repository 方法。
   - 保持实例查询接口 envelope 规范不变。
5. 测试：
   - proto/server 单元测试覆盖消息转换和状态流转。
   - SDK 集成测试启动真实 tonic server，验证 dispatch -> processor -> result。
   - HTTP 测试覆盖 trigger 后实例状态可查询。
6. 文档与交接：
   - 更新 `design/scheduler-architecture-design.md` 开发路线图中的相关完成子项。
   - 更新 `.memory/*`。
   - 生成 `.prompt/010-*.md` 下一阶段提示词。

## 验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun install --cwd web
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
docker build -t scheduler:dev .
docker build -t scheduler-web:dev ./web
```

完成后必须更新路线图、记忆库、后续 prompt，提交并推送。
