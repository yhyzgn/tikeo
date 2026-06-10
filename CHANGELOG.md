# Changelog

This project follows the spirit of [Keep a Changelog](https://keepachangelog.com/) and uses semantic versioning once release lines are published.

## Unreleased

## [0.2.3] - 2026-06-11

### Added

- Reusable Notification Center foundation with explicit `notification_channels`, `notification_policies`, `notification_messages`, and `notification_delivery_attempts` storage, SeaORM migration/entities/repositories, management APIs, OpenAPI registration, and Web `/notifications` console.
- Generic notification delivery worker with retry/DLQ queue status and retry-due processing for webhook-style, Slack, DingTalk, Feishu/Lark, WeCom, PagerDuty, Email, and plugin webhook-compatible providers.
- Job lifecycle notification policies for success, failure, partial failure, cancellation, retry scheduled, retry exhausted, no eligible worker, and script governance failure events.
- English and zh-CN Notification Center and Alerts documentation that separates incident semantics from reusable outbound delivery.

### Changed

- Alerting and Notification Center vocabulary is now enforced in docs/UI: Alerts own rules/events/incidents; Notification Center owns channels/policies/messages/delivery.
- `/notifications` RBAC/menu seed now aligns read access for owner, operator, and viewer while management actions remain permission-gated.
- Release workflow strategy remains tag-driven for `v0.2.x` patch tags; package manifests are synchronized inside release workflows from the pushed tag.

### Fixed

- Prevented Notification Center responses from leaking `secretRefs`, webhook tokens, header credentials such as `X-API-Key`, SMTP passwords, or PagerDuty routing keys.
- Fixed email secret reference alias drift so metadata-aligned `secretRefs.password` and SMTP URL aliases resolve consistently at runtime.
- Fixed retry event semantics: non-retrying terminal failures emit `job_instance.failed`, while `job_instance.retry_exhausted` is reserved for actual exhausted retry policies.
- Improved delivery crash safety by persisting provider result rows before consuming the previous pending attempt, preferring at-least-once delivery over lost notifications.

### Verification

- Local verification passed: Rust fmt/clippy/test/build, CLI smoke, Web lint/typecheck/test/build, docs typecheck/build, workflow/docs/management contract tests, GitHub Actions Node runtime policy, source-size audit, and diff whitespace check.
- Focused code-review subagents reviewed the notification implementation before release; final review passed with the caveat that crash recovery can duplicate delivery after a result row is inserted but before the old attempt is consumed.

### Known gaps

- `notification_templates` storage/API/render endpoints are not implemented yet; `templateRef` is currently a soft link and materialization uses built-in rendering.
- Alert rule backfill/dual-write from `alert_rules.channels_json` into Notification Center policies remains a follow-up.
- Workflow `notification` nodes still need migration from raw channel/target/template fields to registered channel/template references.

## [0.2.0] - 2026-06-08

### Added

- Standalone Docusaurus documentation site with English and zh-CN routes, localized navigation/sidebar/footer/homepage/release content, `llms.txt` entrypoints, and copy-paste deployment runbooks.
- Full SDK documentation coverage for Rust, Go, Java Spring Boot, Python, and Node.js Worker surfaces.
- Production-oriented Helm chart hardening: external database Secret injection, SQLite PVC development path, TLS/mTLS Secret mounts, ingress, probes/resources/security contexts, PodDisruptionBudget, NetworkPolicy, ServiceMonitor, Gateway API Worker Tunnel example, and values schema validation.
- Docker Compose stacks for SQLite, PostgreSQL, and MySQL plus complete Compose YAML publication in the docs.
- README promotion assets and trust surfaces: breathing logo GIF, console tour media, stable badges, SDK runtime badges, full Codecov surface, short console tour, clearer discovery links, star-history entry, support note, and open-source project hygiene files.

### Changed

- CI grouping now reflects runtime ownership across Server, Web, Java/Rust/Go/Python/Node SDK + demo surfaces, deploy tooling, cross-language smoke, Docker validation, source-size policy, and workflow contracts.
- Source modules were split to keep normal Rust/TypeScript source files within the 1500-line maintainability limit enforced by `scripts/check-source-size.py`.
- Documentation defaults to an English root route with zh-CN content under `/zh-CN/`; GitHub Pages project hosting remains available through `TIKEO_DOCS_BASE_URL=/tikeo/`.
- Coverage upload moved to direct Codecov CLI paths across Rust, Web, Java, Go, Python, and Node.js surfaces.
- README and Chinese README now surface the short console tour, clearer discovery links, star-history entry, and support note.

### Fixed

- Corrected CI/coverage badge reliability and SDK runtime version presentation.
- Fixed docs zh-CN route coverage, Chinese language switching, and locale bleed between English and Chinese documentation chrome.
- Prevented unsupported SDK/demo capabilities from being documented as available without verified commands.

### Verification

- Local release preparation includes docs contract/typecheck/build, source-size audit, workflow contract, YAML parse, and targeted locale/route smoke evidence.
- Remote baseline before release: main CI run `27129836559` and coverage run `27129836631` were green for commit `e98f6fd7395f1c104050ce8037db79ab5447aed6`; later docs-only slices were verified locally before release tagging.


## Release notes policy

Each release should include:

- User-facing changes.
- Upgrade notes and breaking changes.
- Verification summary.
- Known gaps or follow-up risks.
