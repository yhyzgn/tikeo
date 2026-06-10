# Audit user guide

The Audit page is implemented by `web/src/pages/AuditLogsPage.tsx`. It is the governance evidence surface for platform write operations, authentication events, script governance actions, dispatch-related events, and failure reasons.

## Source-backed data paths

The page reads `/api/v1/audit-logs` with server-side filters and exports JSON through `/api/v1/audit-logs:export`. The export path keeps the same filters and uses a capped governed export rather than dumping arbitrary database tables.

## Filtering model

Filters include actor, action, resource type, resource id, failure reason, and page size. The URL query state is persistent, so a copied URL can preserve the current audit investigation view. Result tags distinguish successful and failed operations, and failed rows expose the failure reason when present.

## Before/after and trace evidence

Rows may include before/after snapshots, trace IDs, IP address, user agent, and request identifiers. Use trace IDs to correlate API errors with server logs. Use before/after snapshots to verify what actually changed, especially around job scope moves, script publication, API-Key rotation, and RBAC edits.

## Export usage

Export current filters when sharing evidence with operators or release reviewers. The export is JSON, intentionally not CSV, because redaction and content-type policy remain stricter for governance data. Treat exported files as sensitive operational records.
