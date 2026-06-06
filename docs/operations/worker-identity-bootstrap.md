# Worker identity bootstrap

Tikeo workers always initiate outbound gRPC to the Worker Tunnel. A worker must not expose a business inbound port just to receive scheduled work.

## Identity layers

- Worker Pool: scheduling and governance boundary under namespace/app.
- Logical Worker Instance: stable client hint, grouped by `namespace/app/cluster/region/client_instance_id`.
- Worker Session: server-assigned `worker_id` plus generation and fencing token for one tunnel connection.

The server assigns the authoritative `worker_id`. Operators only configure `client_instance_id` and metadata.

## Recommended `client_instance_id`

| Runtime | Recommended value | Notes |
| --- | --- | --- |
| K8s StatefulSet | `${POD_NAME}` or ordinal | Groups restarts by stable slot. |
| K8s Deployment | `${POD_UID}` for each incarnation, or `${POD_NAME}` to group pod restarts | Use labels for worker pool and app. |
| Docker Compose | service/container name plus replica slot | Example: `billing-worker@compose#1`. |
| systemd | `${SERVICE}@${HOST_ID}#${INSTANCE}` | `%H` and `%i` map well to host and template instance. |
| VM/bare metal | `${SERVICE}@${HOST_ID}#${SLOT}` | Prefer inventory/cloud instance id over mutable hostname. |
| Local dev | `${USER}@${HOSTNAME}#dev-${PID}` or fixed demo id | Avoid sharing fixed ids across parallel local processes. |

## Environment template

Use `deploy/worker/identity.env.example` as the shared source of truth for shell/systemd/Compose wrappers:

```bash
cp deploy/worker/identity.env.example ./worker.env
set -a
. ./worker.env
set +a
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

For Spring Boot workers the same values map naturally to properties:

```yaml
tikeo:
  worker:
    endpoint: ${TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}
    client-instance-id: ${TIKEO_WORKER_INSTANCE_ID:billing-worker@host-a#slot-1}
    namespace: ${TIKEO_WORKER_NAMESPACE:default}
    app: ${TIKEO_WORKER_APP:default}
    cluster: ${TIKEO_WORKER_CLUSTER:local}
    region: ${TIKEO_WORKER_REGION:local}
    labels:
      worker_pool: ${TIKEO_WORKER_POOL:default}
```

## Smoke path

1. Start tikeo server and web via Compose or systemd.
2. Run `deploy/smoke/worker-bootstrap-smoke.sh` to verify `/readyz`.
3. Run a worker demo with the same env file.
4. Open Worker UI and confirm sessions appear under lifecycle history by logical id and generation.

Timeout-only sessions must be treated as `lease_expired_unknown`; stream errors use `transport_error`; graceful stops use `graceful_shutdown`; replacements use `replaced_by_new_generation`.
