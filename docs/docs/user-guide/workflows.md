# Workflows user guide

The Workflows page is implemented by `web/src/pages/WorkflowsPage.tsx`. It manages DAG definitions, visual preview/editing, JSON/YAML definition views, validation, dry-run checks, execution, replay, shard inspection, and recovery of workflow nodes.

## Source-backed data paths

The page uses `/api/v1/workflows` for list/create, `/api/v1/workflows/{id}` for read/update, `/api/v1/workflows/{id}/validate` for validation, `/api/v1/workflows/dry-run` for dry-run, and `/api/v1/workflows/{id}/run` to execute. Runtime views also use workflow-instance endpoints and Worker event streams.

## DAG model

A workflow is a `DAG` of nodes and edges. Supported node kinds include job, condition, parallel, join, delay, approval, notification, compensation, map, map_reduce, and sub_workflow. The UI stores visual coordinates inside node config while preserving the executable definition.

## Safe editing flow

Load an existing workflow, use validation before saving, and compare the definition diff. For new workflows, start with a small job-backed DAG and run dry-run before executing. When a node references a job, the job must be in the expected namespace/app and have eligible Workers.

## Runtime triage

After execution, inspect workflow instance state, shards, and replay or recovery results. Recovery actions are operational tools; they should be used after confirming the failed node, the input context, and downstream effects. Use Instances for underlying job execution logs.
