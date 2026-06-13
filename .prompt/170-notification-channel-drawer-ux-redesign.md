# 170 — Notification channel drawer UX redesign

## Scope

Redesign the Notification Center channel create/edit drawer so operators can understand the configuration hierarchy without reading implementation details.

Focus areas:
- Separate scope/provider identity, delivery credentials, channel parameters, message template overrides, and advanced JSON.
- Make `replaceSecretRefs` and `replaceConfig` visibly belong to the sections they control.
- Keep scope selection and Secret candidate filtering as a linked cascade.
- Demote advanced JSON to an explicit escape hatch rather than the primary configuration path.
- Preserve existing backend/API semantics: metadata-only edit by default, no secret echo, and explicit replacement before writing saved provider config or credentials.

## Implementation summary

- `web/src/pages/notifications/ChannelDrawer.tsx` now uses a two-column drawer: left summary/linked scope/test panel, right Step 1-4 plus Advanced section.
- Added a reusable `SectionShell` / `SectionTitle` pattern and live summary helpers for scope path and replacement mode.
- Moved replacement toggles to the delivery credentials and channel parameter section headers.
- Kept the test-send panel tied to saved server-side config and restored the redacted-result safety copy.
- Added Chinese/English i18n entries for the new visible UI copy.
- Added a source-level regression test that locks the new information architecture and replacement-switch placement.

## Verification

- `bun run --cwd web typecheck` ✅
- `bun run --cwd web lint` ✅
- `bun test web/src` ✅ — 151 pass
- `bun run --cwd web build` ✅
- `python3 scripts/check-source-size.py` ✅ — all source files <= 1500 lines
- `git diff --check` ✅

## Guardrails

- This slice intentionally does not change Notification Center APIs or persistence.
- Do not reintroduce raw JSON-first channel editing for built-in providers.
- Do not merge credentials and normal channel config under one replacement switch.
- Future provider-specific fields should extend schema-driven field groups before using Advanced JSON.

## Code review follow-up

A focused code-review pass found no critical issues, but required these fixes before commit:
- Missing English/Chinese i18n for scope help and message-type edit help.
- Advanced JSON warning copy incorrectly implied Advanced overwrote form fields; actual merge order keeps form fields authoritative.
- Create mode showed edit-mode preservation copy.
- Layout source test alone did not protect replacement submit semantics.

Fixes landed:
- Added missing locale entries.
- Reworded Advanced warning to “form fields override advanced JSON matching keys”.
- Added create-mode summary guidance.
- Extracted `buildChannelSubmitPayload` and covered metadata-only edit, replaceConfig, replaceSecretRefs, and redacted-placeholder rejection in `ChannelDrawerPayload.test.ts`.
