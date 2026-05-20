# 进度记录

## 当前状态

- [x] 架构设计文档完成：`design/scheduler-architecture-design.md`
- [x] 移除旧版本/v2 表述，保留功能内容
- [x] 补充多语言动态脚本与安全沙箱设计
- [x] 补充 K8s/Docker/跨集群部署与 Worker Tunnel 网络穿透设计
- [x] 补充 Web UI 与 HTTP/OpenAPI 管理接口设计
- [x] 创建开发阶段总提示词：`prompt.md`
- [x] 初始化 `.memory` 记忆库
- [x] 初始化 `.prompt` 阶段提示词目录
- [x] 固化 Rust workspace + `./crates/` 解耦约束
- [x] 固化 Web 端 `./web` + React + Ant Design + Bun 约束
- [x] 固化依赖尽量使用当前最新稳定版的约束

## 下一大阶段

进入代码开发：`001-bootstrap` 至 `013-broadcast-execution` 已完成；下一阶段执行 `014-worker-capability-routing`。

- [x] 001-bootstrap：初始化 Cargo workspace 与 `./crates/*` crate 骨架
- [x] 001-bootstrap：实现 `scheduler serve`、`/healthz`、`/readyz`
- [x] 001-bootstrap：通过 fmt、clippy、test、build 与 healthz/readyz 冒烟
- [x] 002-http-api-and-openapi：HTTP 管理 API 与 OpenAPI 3.1
- [x] 002-http-api-and-openapi：选择 `utoipa`；禁止 API 文档 UI 依赖
- [x] 002-http-api-and-openapi：实现 `/api/v1/system/info`、`/api/v1/cluster`、Jobs skeleton
- [x] 002-http-api-and-openapi：暴露 `/api-docs/openapi.json`；不提供文档 UI
- [x] 002-http-api-and-openapi：后端入口调整为根 `src/main.rs`，业务模块继续在 `crates/*`
- [x] 003-worker-tunnel：Worker 主动连接与注册心跳
- [x] 固化 HTTP 业务接口统一 `{code,message,data}` 响应规范
- [x] 已在设计文档开发路线图标记完成项：脚手架、HTTP API skeleton、OpenAPI JSON、CLI serve
- [x] 路线图完成项标记规范调整为仅使用 `[x]`，不额外使用 ✅ 图标
- [x] Java SDK 规划补充：优先支持 Spring Boot Starter 模式
- [x] 003-worker-tunnel：新增 `scheduler-proto` crate 与 Worker Tunnel protobuf
- [x] 003-worker-tunnel：实现 server 侧 Worker Tunnel gRPC skeleton 与内存 registry
- [x] 003-worker-tunnel：server 同时启动 HTTP 9090 与 Worker Tunnel gRPC 9091
- [x] 设计路线图标记：gRPC 协议定义与代码生成
- [x] 004-storage-and-scheduler：SeaORM 存储层、SQLite dev DB、MySQL migration feature、Jobs API 持久化
- [x] 005-basic-scheduler：调度领域模型、API 手动触发实例链路、实例列表查询
- [x] 006-worker-sdk-rust-and-java-starter：Rust Worker SDK 注册/心跳客户端 + Java Spring Boot Starter 骨架
- [x] 007-web-ui-foundation：Web 管理端基础工程、Job/Instance 页面骨架
- [x] 008-container-deployment：Docker / Compose / K8s 部署基础
- [x] 009-worker-dispatch：Worker Tunnel 真实任务分发、执行回传与实例状态流转
- [x] 010-scheduler-tick-loop：CRON / Fixed Rate tick loop 与调度触发
- [x] 011-instance-logs：实例执行日志与 Web 日志查看基础
- [x] 012-auth-rbac-foundation：登录与权限感知操作基础
- [x] 013-broadcast-execution：广播执行基础
- [x] 014-worker-capability-routing：Worker 能力 / 标签 / namespace / app 基础路由
- [x] 015-user-management-and-rbac：账号体系、用户管理、RBAC 权限验证与 SessionStore 抽象
- [x] 016-dynamic-script-sandbox：脚本定义 CRUD（storage + migration + repository + HTTP API + OpenAPI）、ScriptLanguage/ScriptStatus 核心类型、Web 脚本管理页面