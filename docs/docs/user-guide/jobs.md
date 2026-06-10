# Jobs user guide

The Jobs page is implemented by `web/src/pages/JobsPage.tsx`. It manages job definitions, schedule type, namespace/app scope, script/plugin/SDK processor bindings, version history, rollback, API-triggered execution, broadcast selector execution, and scheduling advice.

## Source-backed data paths

The page uses `/api/v1/jobs` to list and create jobs, `/api/v1/jobs/{job}` to update or delete a job, `/api/v1/jobs/{job}:trigger` to start an API-triggered instance, `/api/v1/jobs/{job}/versions` and `/api/v1/jobs/{job}/rollback` for version history, and `/api/v1/jobs/{job}/scheduling-advice` for capacity checks. Worker processor choices are refreshed from Worker Tunnel snapshots.

## Creating and editing jobs

Choose namespace and app first because later routing, canary target validation, and service-account access depend on scope. The edit drawer allows scope moves only when the backend authorizes both the source and destination scope. Processor binding should be explicit: SDK processors come from Worker structured capabilities, scripts come from approved scripts, and plugins come from enabled plugin processor definitions.

## Triggering and broadcast execution

The default API trigger path uses `triggerType=api` and `executionMode=single`. Broadcast execution is opt-in through the page's broadcast drawer and the `broadcastSelector` payload. Use tags, region, cluster, or labels only when Workers actually advertise corresponding structured capabilities or labels; do not rely on job-name conventions.

## Validation and troubleshooting

Before saving, compare the selected schedule type, retry policy, calendar, canary target, and worker pool. Before triggering, open scheduling advice to verify eligible workers. After triggering, open Instances and use `/api/v1/instances/{instance}` plus `/api/v1/instances/{instance}/logs` to confirm result and log evidence.
