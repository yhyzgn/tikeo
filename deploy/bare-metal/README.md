# Bare-metal / VM bootstrap 🖥️

[🇨🇳 中文部署文档](../../docs/zh-CN/deployment.md)

Use this path for conventional servers, VMs, Supervisor, or manually managed process runners.

```bash
cargo build --release --bin tikeo
install -d ./var/lib/tikeo ./logs
cp config/dev.toml ./tikeo.toml
./target/release/tikeo serve --config ./tikeo.toml
```

Operational cautions:

- Set `observability.logging.log_dir` to a durable directory.
- Use a stable worker identity such as `${service}@${host}#${slot}` for bare-metal workers.
- Prefer PostgreSQL/MySQL for multi-node services.
- Use TLS/mTLS before exposing HTTP or Worker Tunnel listeners outside a trusted network.
