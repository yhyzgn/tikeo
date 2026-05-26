# Java demo integration report

## Summary

Status: **passed**

The automated integration smoke verified that the tikee server, Java SDK, and Java Spring worker demo can run together through the real outbound Worker Tunnel. The Java demo registered as an online worker and executed API, broadcast, scheduled, and workflow-materialized task dispatches.

## Run evidence

- Report JSON: `.dev/reports/java-demo-20260526T031503Z-101133.json`
- API URL: `http://127.0.0.1:19090`
- Worker Tunnel: `http://127.0.0.1:19998`
- Demo Web URL: `http://127.0.0.1:18080`
- Generated at: `2026-05-26T03:15:21.538519+00:00`

## Passed cases

| Case | Evidence |
| --- | --- |
| Spring Boot Web health | `GET /demo/health` passed on embedded Tomcat at `http://127.0.0.1:18080`. |
| Worker registration | `spring-demo-worker` registered online with Java/Spring Boot capabilities. |
| API single success | `demo.echo` completed instance `inst_019e624779f478239f0a08ef059883ce` as `succeeded`. |
| API single failure | `demo.fail` completed instance `inst_019e62477a287291a3e84332cef0b632` as `failed`. |
| Broadcast success | `demo.context` completed broadcast instance `inst_019e62477a5b77c3a5739fbe36a69e56` as `succeeded`. |
| Fixed-rate success | `demo.heartbeat` completed fixed-rate instance `inst_019e62477bb976b0ad45f24c0c6a5b5b` as `succeeded`. |
| Cron success | `demo.report` completed cron instance `inst_019e62477d177fd393801a4656cea9de` as `succeeded`. |
| Workflow job success | workflow `wf_019e6247929b7ba3a81670b5016717c4` / instance `wfi_019e624792d57723a13adc5b0d3391cf` materialized job instance `inst_019e6247931f7203963a3205c61c6d79` and reached `succeeded`. |

## Additional verification

- Java SDK package-level tests: `(cd sdks/java && ./gradlew test --no-daemon)` passed.
- Java Spring demo tests: `(cd examples/java/spring-worker-demo && ./gradlew test --no-daemon)` passed.
- Integration smoke: `deploy/smoke/java-demo-integration-smoke.sh` passed.

## Known boundaries

- This report covers local plaintext dev server integration, not TLS/mTLS or external DB deployment.
- Java SDK still intentionally rejects WASM/script processor bindings; this smoke covers Java annotation processors for normal Worker Tunnel tasks.
- Python/Node SDKs and Go run-loop remain deferred.
