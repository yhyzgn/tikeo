# 063 — Phase 3 audit export governance

## Context
Audit logs now include before/after snapshots, trace_id, result, and failure_reason in storage/API/Web. Existing list API supports filters and pagination.

## Required next work
1. Add governed audit export support, preferably `GET /api/v1/audit-logs:export` with explicit format (`json` or `csv`) and the same filters as list.
2. Keep exports permission-gated by `audit:read`; add guardrails for maximum rows, stable ordering, and redaction guidance for sensitive before/after/detail fields.
3. Ensure responses still use `{ code, message, data }` for JSON exports; if CSV is implemented, document content-type and envelope exception only if truly necessary.
4. Update Web UI audit page with an export action if backend support is implemented.
5. Preserve constraints: no database foreign keys, no Swagger UI, no secrets in repo, API envelope by default.
6. Update `design/tikeo-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, and create `.prompt/064-*.md`.
7. Run full verification, commit with Lore-style trailers, and push.
