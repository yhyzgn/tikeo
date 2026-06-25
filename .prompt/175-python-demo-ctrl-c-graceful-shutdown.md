# 175 — Python demo Ctrl+C graceful shutdown follow-up

Context:
- User reported Python worker demo prints a traceback when pressing Ctrl+C while blocked inside gRPC stream iteration (`session.process_next(...)`).
- Project red line remains: do not add compiler/linter/typecheck suppressions to hide warnings or errors.

Implemented:
- `examples/python/worker-demo/src/tikeo_python_worker_demo/__main__.py`
  - Wrapped live reconnect/processing loop with `except KeyboardInterrupt`.
  - Preserved cleanup: heartbeat stop event is set and session is closed from `finally`.
  - Session close failures are logged and do not mask the Ctrl+C shutdown path.
- `examples/python/worker-demo/tests/test_demo.py`
  - Added regression test simulating `KeyboardInterrupt` from `process_next`.
  - Verifies heartbeat stop, session close, and clean shutdown log.

Verification:
- `python -m pytest examples/python/worker-demo/tests/test_demo.py sdks/python/tikeo/tests/test_sdk.py -q` => 26 passed.
- Dry-run demo prints expected registration payload and `dry_run_heartbeat_sequence=1`.
- Simulated live KeyboardInterrupt path exits without `Traceback` or leaked `KeyboardInterrupt`.
- SDK/examples suppression scan returns zero matches.
- `git diff --check` passes.

Next:
- Commit/push the fix and track CI to completion.
