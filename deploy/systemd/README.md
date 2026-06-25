# systemd deployment ⚙️

[🇨🇳 中文部署文档](../../README.zh-CN.md#运行-tikeo-服务)

Systemd units run Tikeo as a normal Linux service with explicit state, config, and log directories.

```bash
sudo useradd --system --home /var/lib/tikeo --shell /usr/sbin/nologin tikeo || true
sudo install -d -o tikeo -g tikeo /opt/tikeo/bin /var/lib/tikeo /var/log/tikeo /etc/tikeo
sudo install -m 0755 target/release/tikeo /opt/tikeo/bin/tikeo
sudo install -m 0644 config/tikeo.yml /etc/tikeo/tikeo.yml
sudo install -m 0644 deploy/systemd/tikeo.env /etc/tikeo/tikeo.env
sudo install -m 0644 deploy/systemd/tikeo.service /etc/systemd/system/tikeo.service
sudo systemctl daemon-reload
sudo systemctl enable --now tikeo
```

Use the worker unit templates as examples for stable `client_instance_id` values and outbound-only
Worker Tunnel connectivity.
