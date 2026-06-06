# Phase 4 P2 plugin system closed loop

This phase implements the P2 plugin system item for custom processor types and custom alert channels.

Implemented boundaries:
- Storage: `plugins` table with processor type declarations and alert channel type declarations.
- HTTP: `/api/v1/plugins` list/create/update/delete, guarded by tenant read/manage permissions.
- Jobs: `processorType` field added to create/update/summary/storage/version snapshots. Custom processor types are validated against the plugin registry, and `processorName` must come from the selected type's `processorNames` candidates.
- Scheduling/dispatch: scheduling advice and Worker dispatch use structured Worker requirements and match `pluginProcessors.type + processorNames`; legacy `plugin-processor:<type>` strings are compatibility-only fallbacks, not the contract.
- Alerts: plugin alert channel types are visible in delivery readiness and can be materialized into webhook-compatible notification channels with simple `{{message}}`, `{{resource_id}}`, `{{resource_type}}`, `{{severity}}` template replacement.
- Web: `/plugins` management page, menu route, API client types, and Jobs create/edit plugin processor selector.
- Demo: Java Spring worker declares `@TikeoProcessor(value = "billing.sql-sync", kind = PLUGIN, pluginType = "sql")`, producing structured `pluginProcessors` registration. Rust demo remains compatibility/compile-safe and should not be treated as the source of the new contract.

Validation anchors:
- `cargo test -p tikeo-storage plugin_repository_resolves_custom_processor_and_alert_channel_types -- --nocapture`
- `cargo test -p tikeo-server plugin_registry_supports_custom_processor_types_and_alert_channels -- --nocapture`
- `cd web && bun test src/pages/__tests__/PluginsPage.test.tsx`
- `cd web && bun run lint && bun run build`

Next phases should not reintroduce hard-coded custom processor enums or string-convention capability contracts. Use the plugin registry plus Worker structured capabilities as the source of truth for UI choices and dispatch matching.
