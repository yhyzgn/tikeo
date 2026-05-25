# tikee deployment bootstrap

P0 deployment support focuses on service usability for Docker Compose, systemd, and bare-metal/VM hosts. Helm remains deferred until external DB, gateway, secrets, and TLS parameters stabilize.

## Layout

- `compose/` — Compose overlay examples for local production-like startup.
- `systemd/` — systemd server and worker unit/environment templates.
- `bare-metal/` — direct binary install and smoke-check helpers.
- `worker/` — shared Worker identity environment template.
- `smoke/` — smoke checks that avoid inbound business ports.

## Quick paths

```bash
# Compose with the root docker-compose.yml plus production-minded env file
cp deploy/compose/tikee.env.example .env
DOCKER_BUILDKIT=1 docker compose --env-file .env up -d --build

# Readiness and worker bootstrap smoke
TIKEE_HTTP_URL=http://127.0.0.1:9090 ./deploy/smoke/worker-bootstrap-smoke.sh

# Bare metal config check
./deploy/bare-metal/check-config.sh config/dev.toml

# systemd server install sketch
sudo install -m 0755 target/release/tikee /opt/tikee/bin/tikee
sudo install -m 0644 deploy/systemd/tikee.env /etc/tikee/tikee.env
sudo install -m 0644 deploy/systemd/tikee.service /etc/systemd/system/tikee.service
sudo systemctl daemon-reload && sudo systemctl enable --now tikee
```

Worker identity guidance lives in `docs/operations/worker-identity-bootstrap.md`.
