# Tikeo 部署中文入口 🚢

Tikeo 支持本地、容器、传统服务器和 Kubernetes 部署路径：

- Docker Compose：默认 SQLite，并提供 PostgreSQL / MySQL 覆盖文件。
- systemd / bare metal：适合 VM、物理机和受控进程管理器。
- Helm：面向 Kubernetes 发布。
- K8s CRD/operator：用于 GitOps 漂移检查和状态回写。
- Terraform Provider：用于 manifest 导出和 diff 资源。

生产环境建议使用 PostgreSQL 或 MySQL，并开启明确的日志目录、指标采集、OpenTelemetry tracing、TLS/mTLS 和外部 Secret 管理。
