# systemd bootstrap

## Server

1. Build or install the `tikee` binary to `/opt/tikee/bin/tikee`.
2. Create a dedicated `tikee` user and writable state directory.
3. Copy `deploy/systemd/tikee.env` to `/etc/tikee/tikee.env`.
4. Copy a config file to `/etc/tikee/tikee.toml` and point storage to SQLite or an external DB.
5. Install `deploy/systemd/tikee.service` to `/etc/systemd/system/tikee.service`.

```bash
sudo useradd --system --home /var/lib/tikee --shell /usr/sbin/nologin tikee || true
sudo install -d -o tikee -g tikee /opt/tikee/bin /var/lib/tikee /etc/tikee
sudo install -m 0755 target/release/tikee /opt/tikee/bin/tikee
sudo install -m 0644 config/container.toml /etc/tikee/tikee.toml
sudo install -m 0644 deploy/systemd/tikee.env /etc/tikee/tikee.env
sudo install -m 0644 deploy/systemd/tikee.service /etc/systemd/system/tikee.service
sudo systemctl daemon-reload
sudo systemctl enable --now tikee
```

## Worker template

The Rust demo worker unit shows the identity pattern for real workers:

```bash
cargo build --release --manifest-path examples/rust/worker-demo/Cargo.toml
sudo install -m 0755 examples/rust/worker-demo/target/release/tikee-rust-worker-demo /opt/tikee/bin/tikee-rust-worker-demo
sudo install -m 0644 deploy/systemd/tikee-worker-rust-demo.env /etc/tikee/tikee-worker-rust-demo.env
sudo install -m 0644 deploy/systemd/tikee-worker-rust-demo@.service /etc/systemd/system/tikee-worker-rust-demo@.service
sudo systemctl daemon-reload
sudo systemctl enable --now tikee-worker-rust-demo@slot-1
```

Use `%H` plus `%i` to build stable `client_instance_id` values like `rust-demo-worker@host-a#slot-1`.
