# Bare-metal / VM bootstrap

Use this path for conventional servers, VMs, Supervisor, or manually managed process runners.

```bash
cargo build --release --bin tikee
install -d ./var/lib/tikee
cp config/dev.toml ./tikee.toml
./target/release/tikee serve --config ./tikee.toml
```

Smoke-check a config without installing systemd:

```bash
./deploy/bare-metal/check-config.sh config/dev.toml
```

Operational recommendations:

- Use a stable hostname or CMDB/cloud instance id in Worker `client_instance_id` values for bare-metal workers.
- Prefer PostgreSQL/MySQL for shared services; SQLite is acceptable for single-node development or small demos.
- Keep HTTP and Worker Tunnel ports separately firewalled; Worker Tunnel is outbound-only from workers to server.
- Use TLS/mTLS config when exposing either listener beyond a trusted private network.
