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
TIKEE_HTTP_URL=http://127.0.0.1:9090 ./deploy/smoke/worker-bootstrap-smoke.sh
```

Run a dry worker identity smoke:

```bash
set -a
. deploy/worker/identity.env.example
set +a
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Operational recommendations:

- Use `${service_name}@${host_id}#${instance_slot}` as Worker `client_instance_id` for bare-metal/systemd workers.
- Prefer explicit `TIKEE_WORKER_HOST_ID` from inventory or cloud metadata; fall back to `/etc/machine-id` or stable hostname only when necessary.
- Prefer PostgreSQL/MySQL for shared services; SQLite is acceptable for single-node development or small demos.
- Keep HTTP and Worker Tunnel ports separately firewalled; Worker Tunnel is outbound-only from workers to server.
- Use TLS/mTLS config when exposing either listener beyond a trusted private network.
