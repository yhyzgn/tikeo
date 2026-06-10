# Instances user guide

The Instances page is implemented by `web/src/pages/InstancesPage.tsx`. It shows the execution records produced by Jobs and Workflows, including per-attempt nodes, broadcast results, logs, cancellation, and live stream updates.

## Source-backed data paths

The page loads jobs through `/api/v1/jobs`, instances through `/api/v1/jobs/{job}/instances`, details through `/api/v1/instances/{instance}`, attempts through `/api/v1/instances/{instance}/attempts`, and logs through `/api/v1/instances/{instance}/logs`. It also listens to `/api/v1/instances/stream` and can open a log stream for a selected instance.

## Reading status

Status values are shown as tags. `pending` means the scheduler has not completed dispatch. `running` means a Worker has accepted work or logs are arriving. `succeeded`, `failed`, and `partial_failed` are terminal evidence states; broadcast instances can be partial when some execution nodes succeed and others fail.

## Logs and execution nodes

For single-worker execution, the page displays the selected Worker from the instance result or latest log. For broadcast execution, `InstancesPage.tsx` builds an execution result node per attempt and groups logs by `workerId`. Use the copyable Worker ID to compare with the Workers page and Worker lifecycle history.

## Cancellation boundary

Cancellation uses the Management API, not a browser-only state toggle. Cancel only when the instance is still active and RBAC allows execution control. After canceling, refresh the detail drawer and inspect logs because a Worker can report final cleanup or failure evidence after the request is accepted.
