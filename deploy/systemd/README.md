# systemd bootstrap

## Server

1. Build or install the `tikeo` binary to `/opt/tikeo/bin/tikeo`.
2. Create a dedicated `tikeo` user and writable state directory.
3. Copy `deploy/systemd/tikeo.env` to `/etc/tikeo/tikeo.env`.
4. Copy a config file to `/etc/tikeo/tikeo.toml` and point storage to SQLite or an external DB.
5. Install `deploy/systemd/tikeo.service` to `/etc/systemd/system/tikeo.service`.

```bash
sudo useradd --system --home /var/lib/tikeo --shell /usr/sbin/nologin tikeo || true
sudo install -d -o tikeo -g tikeo /opt/tikeo/bin /var/lib/tikeo /etc/tikeo
sudo install -m 0755 target/release/tikeo /opt/tikeo/bin/tikeo
sudo install -m 0644 config/container.toml /etc/tikeo/tikeo.toml
sudo install -m 0644 deploy/systemd/tikeo.env /etc/tikeo/tikeo.env
sudo install -m 0644 deploy/systemd/tikeo.service /etc/systemd/system/tikeo.service
sudo systemctl daemon-reload
sudo systemctl enable --now tikeo
```

## Worker template

The Rust demo worker unit shows the identity pattern for real workers:

```bash
cargo build --release --manifest-path examples/rust/worker-demo/Cargo.toml
sudo install -m 0755 examples/rust/worker-demo/target/release/tikeo-rust-worker-demo /opt/tikeo/bin/tikeo-rust-worker-demo
sudo install -m 0644 deploy/systemd/tikeo-worker-rust-demo.env /etc/tikeo/tikeo-worker-rust-demo.env
sudo install -m 0644 deploy/systemd/tikeo-worker-rust-demo@.service /etc/systemd/system/tikeo-worker-rust-demo@.service
sudo systemctl daemon-reload
sudo systemctl enable --now tikeo-worker-rust-demo@slot-1
```

Use `%H` plus `%i` to build stable `client_instance_id` values like `rust-demo-worker@host-a#slot-1`.
