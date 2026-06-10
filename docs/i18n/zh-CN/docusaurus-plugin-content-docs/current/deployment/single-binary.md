---
title: 单二进制与 systemd
description: Tikeo Server 单二进制、本地运行、VM/systemd 部署与回滚命令。
---

# 单二进制与 systemd

这是最小部署路径：一个 `tikeo` Server 二进制、一个配置文件、一个持久化数据目录。排障比容器和 Kubernetes 更直接，适合首次评估或 VM 部署。

## 本地一条命令启动

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

另开终端检查：

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
```

`config/dev.toml` 默认 HTTP `9090`、Worker Tunnel `9998`，并使用仓库工作目录下的 SQLite 文件。

## 构建 release 二进制

```bash
cargo build --release --bin tikeo
install -d ./var/lib/tikeo ./logs
cp config/dev.toml ./tikeo.toml
./target/release/tikeo serve --config ./tikeo.toml
```

暴露到外部网络前，请先调整 `./tikeo.toml`：选择持久化数据库路径或外部数据库 URL，设置日志目录，并在跨主机/跨集群时启用 TLS/mTLS。

## systemd 复制即用安装

```bash
cargo build --release --bin tikeo
sudo useradd --system --home /var/lib/tikeo --shell /usr/sbin/nologin tikeo || true
sudo install -d -o tikeo -g tikeo /opt/tikeo/bin /var/lib/tikeo /var/log/tikeo /etc/tikeo
sudo install -m 0755 target/release/tikeo /opt/tikeo/bin/tikeo
sudo install -m 0644 config/container.toml /etc/tikeo/tikeo.toml
sudo install -m 0644 deploy/systemd/tikeo.env /etc/tikeo/tikeo.env
sudo install -m 0644 deploy/systemd/tikeo.service /etc/systemd/system/tikeo.service
sudo systemctl daemon-reload
sudo systemctl enable --now tikeo
sudo systemctl status tikeo --no-pager
```

Smoke：

```bash
curl -fsS http://127.0.0.1:9090/readyz
journalctl -u tikeo -n 80 --no-pager
```

## 最小参数检查

| 参数 | 本地默认 | 生产建议 |
|---|---|---|
| `server.listen_addr` | `0.0.0.0:9090` | 放在代理或防火墙后；直连暴露时启用 HTTP TLS。 |
| `server.worker_tunnel_addr` | `0.0.0.0:9998` | 只发布 Server tunnel endpoint；Worker 仍主动出站连接。 |
| `storage.database_url` | SQLite | 共享环境用 PostgreSQL/MySQL/CockroachDB。 |
| `observability.logging.level` | `info` | 保持 `info`，VM 部署加 `log_dir`。 |
| `transport_security.worker_tunnel` | 开发态关闭 | 跨信任边界前启用 TLS/mTLS。 |

## 回滚

systemd 回滚就是安装旧二进制并重启服务。数据库 migration 不会被 systemd 回滚，共享环境升级前必须先做数据库快照。

```bash
sudo install -m 0755 ./previous/tikeo /opt/tikeo/bin/tikeo
sudo systemctl restart tikeo
curl -fsS http://127.0.0.1:9090/readyz
```

## 适用边界

单二进制部署适合小规模内部服务、开发环境、演示环境和需要人工控制升级节奏的 VM。它不提供 Kubernetes 的自动调度、滚动发布或 Secret 管理能力；如果需要多副本、NetworkPolicy、Ingress、ServiceMonitor 或 Gateway API，应使用 Helm 路径。
