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

进入代码开发：从 `001-bootstrap` 开始初始化 Rust workspace、基础 crate、CI/命令和最小可验证服务骨架。
