# 049 — Phase 3 audit trace/failure governance

## Context
Phase2 is closed with safe distributed foundations and real Raft runtime explicitly deferred because openraft is still `0.10.0-alpha.20`. Phase3 audit governance has started: audit list now supports server-side actor/action/resource filters, page_size/page_token, and total count; Web UI has filter controls.

## Goal
Continue audit governance by adding trace/failure visibility without breaking the `{code,message,data}` API envelope.

## Required work
1. Add a generated trace id to API error envelopes instead of the current `unavailable` placeholder.
2. Consider recording failed write-operation attempts in audit logs where authorization/storage validation reaches route code.
3. Preserve existing successful audit behavior and avoid logging sensitive passwords/tokens.
4. Update Web UI to show trace ids where useful if API exposes them.
5. Update design/.memory/roadmap.

## Constraints
- No database foreign keys.
- Do not log secrets.
- Keep API envelope `{code,message,data}`.
- Raft runtime remains deferred unless a real consensus proof is implemented.

## Validation
- cargo fmt/clippy/test and web typecheck/build if touched.
- Commit and push with Lore trailers.
