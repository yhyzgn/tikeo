# Changelog

This project follows the spirit of [Keep a Changelog](https://keepachangelog.com/) and uses semantic versioning once release lines are published.

## Unreleased

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
