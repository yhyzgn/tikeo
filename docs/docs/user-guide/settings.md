# Settings and governance guide

The Settings-related console surfaces are defined by `web/src/routes.tsx` rather than one monolithic settings page. Current routes include users, roles, tenant scopes, API-Key management, calendars, GitOps/IaC, OIDC identities where enabled, and observability/governance entries.

## Route and RBAC source

`web/src/routes.tsx` is the single route metadata source. Menu entries declare paths, labels, icon groups, and RBAC resource/action requirements. The same RBAC model hides unavailable actions and routes, so an operator should first check permissions when a settings page is missing.

## API-Key management

API-Key and service-account management is app-scoped machine-to-machine governance. SDK Management API calls use `x-tikeo-api-key`, not a human browser session. Rotate or revoke keys from the API-Key route when a credential is no longer needed or after exposure.

## Tenant scopes and roles

Namespace/app scope controls job ownership, service accounts, Worker pools, secrets, and canary targets. RBAC roles control which users can read, write, execute, or manage each resource family. Avoid moving jobs across scopes unless the user is authorized for both the source and destination.

## Operational boundary

Settings routes are not placeholders for hidden features. If a route is disabled, it is not production-ready. If a page exists but an action is hidden, use the required `RBAC` resource/action from route metadata and permission catalog to decide whether to grant access or keep the action unavailable.
