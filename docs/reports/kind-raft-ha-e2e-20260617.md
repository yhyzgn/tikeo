# Kind Raft HA E2E Evidence — 2026-06-17

Local raw evidence directory:

```text
.dev/reports/kind-raft-ha-e2e-20260617T053254Z-3621444/
```

Final smoke report:

```text
.dev/reports/kind-raft-ha-e2e-20260617T053254Z-3621444/kind-raft-ha-e2e-20260617T053254Z-3621444.json
```

Summary:

- Status: `passed`
- Cases: `19`
- Kind node image: `kindest/node:v1.33.1`
- Server replicas: `4`
- Initial leader: `tikeo-server-0`
- Post-failover leader: `tikeo-server-1`
- API pod used by the SDK/API path: `tikeo-server-1`
- Worker Tunnel gateway pod: `tikeo-server-2`
- Before-failover rollout gate: active shard rows `64`, active owners `4`, ownership skew `0`, outbox `completed=1`
- Fault drill: deleted `tikeo-server-0`, StatefulSet recovered, postcheck passed

Important local evidence files:

- `cluster-diagnostics-before-failover.json`
- `metrics-summary-before-failover.json`
- `rollout-before-failover.json`
- `fault-drill/precheck.json`
- `fault-drill/postcheck.json`
- `rollout-after-failover.json`
- `db-evidence-before-failover.json`
- `db-evidence-after-failover.json`
- `instance-result-before-failover.json`
- `instance-result-after-failover.json`
- `service-probe-initial.log`
- `service-probe-after-failover.log`
- `worker.log`
- `pods-final.txt`

Validation commands also run:

```bash
bash -n scripts/kind-raft-ha-e2e.sh scripts/verify-raft-ha-rollout.sh scripts/raft-ha-fault-injection-drill.sh
cargo fmt --all -- --check
cargo test -p tikeo-server raft_static_bootstrap_preserves_removed_members -- --nocapture
cargo test -p tikeo-server raft_config_persists_bootstrap_metadata_but_remains_unschedulable -- --nocapture
cargo clippy -p tikeo-server --all-targets -- -D warnings
npm --prefix docs run build
git diff --check
```
