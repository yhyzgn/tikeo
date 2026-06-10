---
title: Single binary and systemd
description: Copy-paste local and VM deployment paths for the Tikeo Server binary.
---

# Single binary and systemd

Use this path when you want the smallest possible Tikeo deployment: one `tikeo` Server binary, one config file, and a durable data directory. It is the best first step before Docker or Kubernetes because failures are easier to see.

## One-command local evaluation

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

In another terminal:

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
```

`config/dev.toml` listens on HTTP `9090`, Worker Tunnel `9998`, and uses `sqlite://tikeo-dev.db?mode=rwc` in the repository working directory.

## Build a release binary

```bash
cargo build --release --bin tikeo
install -d ./var/lib/tikeo ./logs
cp config/dev.toml ./tikeo.toml
./target/release/tikeo serve --config ./tikeo.toml
```

For a VM, edit `./tikeo.toml` before exposing it outside localhost. At minimum choose a durable database path or external database URL, set log retention, and enable TLS/mTLS before putting the Worker Tunnel on an untrusted network.

## Copy-paste systemd install

This command sequence uses the committed service unit in `deploy/systemd/tikeo.service` and environment file in `deploy/systemd/tikeo.env`.

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

Smoke:

```bash
curl -fsS http://127.0.0.1:9090/readyz
journalctl -u tikeo -n 80 --no-pager
```

## Minimum config checklist

| Setting | Local value | Production guidance |
|---|---|---|
| `server.listen_addr` | `0.0.0.0:9090` | Bind behind a proxy or firewall; enable HTTP TLS if exposed directly. |
| `server.worker_tunnel_addr` | `0.0.0.0:9998` | Publish only the Server tunnel endpoint; workers still connect outbound. |
| `storage.database_url` | SQLite file URL | Use PostgreSQL/MySQL/CockroachDB for shared environments. |
| `observability.logging.level` | `info` | Keep `info`; add `log_dir` for durable VM logs. |
| `transport_security.worker_tunnel` | TLS off in dev | Enable TLS/mTLS before crossing trust boundaries. |

## Rollback

For binary/systemd deployments, rollback means reinstalling the previous binary and restarting the service. Database migrations are not reversed by systemd, so take a database snapshot before changing versions in shared environments.

```bash
sudo install -m 0755 ./previous/tikeo /opt/tikeo/bin/tikeo
sudo systemctl restart tikeo
curl -fsS http://127.0.0.1:9090/readyz
```
