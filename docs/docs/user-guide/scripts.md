# Scripts user guide

The Scripts page is implemented by `web/src/pages/ScriptsPage.tsx`. It manages script drafts, execution policy, approval/publish flow, rollback, version history, and source/policy `diff` previews.

## Source-backed data paths

The page uses `/api/v1/scripts` for list/create, `/api/v1/scripts/{id}` for read/update/delete, `/api/v1/scripts/{id}/publish`, `/api/v1/scripts/{id}/rollback`, `/api/v1/scripts/{id}/versions`, and `/api/v1/scripts/{id}/diff`. Jobs can bind to approved scripts after publication.

## Execution policy

The form exposes timeout, memory, output limit, environment variables, filesystem, network, secrets, and sandbox backend. The safe default is deny-by-default network/filesystem and bounded resources. Workers must advertise a matching executable script runner before scripts should be scheduled to them.

## Draft, diff, publish, rollback

Use the preview action to inspect the content and policy diff before saving or publishing. Publishing creates an immutable version used by dispatch. Rollback returns to a previous approved version and should be treated like a production change because running jobs may bind to the new release pointer.

## Boundaries

The Server dispatches script metadata and immutable content; it does not execute user code. Execution happens inside Worker-controlled sandboxes. If no capable Worker exists, fix Worker runtime capabilities rather than widening policy or pretending the script can run.
