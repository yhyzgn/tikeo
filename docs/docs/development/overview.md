---
title: Development and extension guide
description: How to work on Tikeo safely, including repository structure, build and test loops, API changes, Web UI changes, SDK changes, docs updates, and extension boundaries.
keywords: [tikeo development, contributor guide, sdk development, notification provider, web development]
---

# Development and extension guide

This page is for maintainers and teams extending Tikeo. It explains the repository shape, how to make changes without breaking runtime contracts, which tests prove each layer, and where to document new behavior. It is intentionally practical: if you change one subsystem, you should know the adjacent files, tests, docs, and release checks that must move with it.

## Repository map

| Path | What belongs here | Notes |
| --- | --- | --- |
| `crates/tikeo-config` | Config structures, defaults, environment override behavior | Update docs when defaults change. |
| `crates/tikeo-server` | HTTP routes, Worker Tunnel service, scheduling, notifications, auth, CLI | Keep runtime behavior real; no placeholder branches. |
| `crates/tikeo-storage` | Entities, migrations, repositories | Add compatibility tests for database-sensitive changes. |
| `crates/tikeo-proto` | Worker Tunnel protobuf and generated bindings | Update SDKs and protocol reference when changed. |
| `web/` | React/TypeScript/Bun operator console | Use Bun and keep UI text covered by i18n tests. |
| `docs/` | Docusaurus docs site and docs Docker image | Docs must be human-readable and verified against code. |
| `sdks/` | Rust/Go/Java/Python/Node SDKs | Keep Worker and Management contracts aligned. |
| `examples/` | Runnable Worker demos | Demos should be usable smoke targets. |
| `deploy/` | Compose, Helm, K8s, Terraform, systemd, smokes | Keep deployment docs and tests aligned. |

## Local development loop

Use the smallest loop that proves your change:

```bash
cargo fmt --all -- --check
cargo test -p tikeo-server <test-name> --all-features -- --nocapture
bun test web/src/pages/__tests__/NotificationCenterPage.test.tsx
```

Before claiming completion for broad changes:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
bun test web/src
bun run --cwd web typecheck
bun run --cwd web lint
bun run --cwd web build
python3 scripts/check-source-size.py
git diff --check
```

Docs changes should also run:

```bash
python3 .github/tests/docs_site_contract_test.py
cd docs && bun run docs:typecheck && bun run docs:build
```

## API change workflow

When adding or changing an API route:

1. Update DTOs and route handler in `crates/tikeo-server/src/http/routes/*`.
2. Update router wiring in `crates/tikeo-server/src/http/router.rs` if needed.
3. Update OpenAPI generation in `crates/tikeo-server/src/http/openapi.rs`.
4. Add server tests for success, auth failure, validation failure, and tenant/scope guard.
5. Update Web API client under `web/src/api` if the console uses it.
6. Update docs reference and user guide pages.
7. Add contract tokens only when they protect a real regression.

Do not document an endpoint before the route and tests exist.

## Web UI change workflow

Web code uses React, TypeScript, Vite, Ant Design, and Bun. Rules:

- Use `bun`/`bunx` for frontend commands.
- Keep UI copy translatable; update zh-CN/en-US dictionaries.
- Prefer module-level file splitting when files grow.
- Add source-level regression tests for important UX contracts.
- Run `bun test web/src`, `bun run --cwd web typecheck`, `bun run --cwd web lint`, and `bun run --cwd web build` for meaningful UI changes.

For Notification Center UI, preserve the boundary: channels/templates/policies/messages/delivery attempts are Notification Center concepts; alert firing/recovery/silence semantics remain Alerts concepts.

## SDK change workflow

When changing Worker Tunnel or Management API behavior:

1. Update Rust source and protocol first.
2. Update each SDK helper or document the SDK-specific limitation.
3. Update examples so at least one demo can prove the path.
4. Run language-specific tests.
5. Update SDK docs with dependency coordinates, config defaults, minimal Worker, Management create/trigger, and live verification.

Cross-language helper names must remain discoverable in docs: `ManagementClient`, `NewManagementClient`, `HttpTikeoJobClient`, `apiJob`, `apiTrigger`, `broadcastApiTrigger`, and `BroadcastSelectorRequest`.

## Adding a notification provider

A provider is not complete until all layers exist:

- Provider metadata and template schema.
- Config/secret validation.
- Redaction rules for sensitive target data.
- Delivery renderer/sender or plugin boundary.
- Test-send behavior if the provider can be safely tested.
- Web drawer schema labels and help text.
- Docs table entries and troubleshooting.

Never return raw webhook URLs, routing keys, signing keys, SMTP passwords, authorization headers, or token-like values in API summaries.

## Adding a script or sandbox capability

Script capability must fail closed. Before advertising a capability:

- Resolve the runtime tool path.
- Validate input and policy limits.
- Capture stdout/stderr/task logs.
- Report clear unsupported-tool errors.
- Avoid claiming Docker/Podman/WASM/Deno/SRT support unless the Worker can execute that backend.

## Documentation workflow

Docs are product surface. For every feature:

- Add or update the user path, not only the reference page.
- Include prerequisites, commands, expected observations, troubleshooting, and production checklist.
- Avoid internal handoff language in public docs.
- Link to exact reference pages when readers need API details.
- Update zh-CN mirrors for priority docs.
- Run docs contract tests and Docusaurus build.

## Release readiness

Before tagging a release:

- Workspace version and lockfile versions must match the tag.
- Server binary `--version` must report the release version.
- Server, Web, and Docs Docker images should build and publish with the same tag.
- SDK release workflows should use the same version contract.
- GitHub Actions should pass for CI, coverage, release assets, Docker images, and SDK publish jobs.

## Prerequisites

- Rust, Bun, Docker, and any language SDK toolchain touched by your change.
- Local database or isolated smoke environment.
- Understanding of the Server/Worker boundary.
- A plan for docs and tests before broad refactors.

## Verify

Pick the verification set by changed surface:

| Surface | Minimum verification |
| --- | --- |
| Rust server/storage | `cargo fmt`, targeted tests, clippy, workspace tests for broad changes |
| Web | targeted `bun test`, typecheck, lint, build |
| Docs | docs contract, docs typecheck, docs build |
| SDK | language tests plus management trigger smoke when API behavior changes |
| Deploy | Compose/Helm render plus relevant smoke scripts |

## Troubleshooting

| Problem | Response |
| --- | --- |
| Tests pass but docs fail | The public contract changed; update docs or fix the test if it protects obsolete behavior. |
| Web i18n test fails | Add translations for visible text, placeholders, labels, and aria labels. |
| Source-size check fails | Split files by responsibility instead of raising limits. |
| SDK helper drift | Compare language SDK helper names and examples, then update docs and tests together. |

## Production checklist

- [ ] Real behavior is implemented and tested; no placeholders are presented as complete.
- [ ] Runtime, Web, SDK, docs, and deployment surfaces are aligned.
- [ ] Public docs explain how humans deploy, integrate, verify, and troubleshoot the feature.
- [ ] Release/version changes are reflected in binary, lockfile, images, and workflows.
