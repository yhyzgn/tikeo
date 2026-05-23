# 112 — Phase 3 alert retry background worker

## Goal
Close the continuous alert retry scheduling gap by wiring persisted retry/DLQ processing into the server runtime.

## Scope
- Add process configuration for a bounded alert retry worker.
- Start the worker with the server listeners when enabled.
- Gate retry scans on cluster scheduling ownership so Raft followers do not process shared retry state.
- Keep production delivery policy safe by default and retain the manual retry endpoint for on-demand processing.

## Out of scope
- External provider live smoke tests.
- Production SMTP TLS/auth and secret-backed credentials.
