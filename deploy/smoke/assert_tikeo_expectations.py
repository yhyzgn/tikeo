#!/usr/bin/env python3
"""Field-level functional assertions for tikeo smoke tests.

The tool intentionally fails when evidence is missing. It is used by smoke
scripts to prove business expectations, not just HTTP 2xx responses.
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any


class AssertionFailure(Exception):
    pass


def load_json(path: str) -> Any:
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def data(payload: Any) -> Any:
    if isinstance(payload, dict) and "data" in payload:
        return payload["data"]
    return payload


def get_any(obj: dict[str, Any], *names: str, default: Any = None) -> Any:
    for name in names:
        if name in obj:
            return obj[name]
    return default


def items(payload: Any) -> list[dict[str, Any]]:
    value = data(payload)
    if isinstance(value, dict):
        value = value.get("items", [])
    if not isinstance(value, list):
        raise AssertionFailure("expected JSON data/items to be a list")
    return [item for item in value if isinstance(item, dict)]


def text_blob(payload: Any) -> str:
    return json.dumps(payload, ensure_ascii=False, sort_keys=True)


def fail(message: str) -> None:
    raise AssertionFailure(message)


def assert_workers(args: argparse.Namespace) -> dict[str, Any]:
    workers = items(load_json(args.file))
    if args.min_online is not None:
        online = [w for w in workers if str(get_any(w, "status", default="")).lower() == "online"]
        if len(online) < args.min_online:
            fail(f"expected at least {args.min_online} online workers, got {len(online)}")
    if args.client_instance:
        for expected in args.client_instance:
            found = [w for w in workers if get_any(w, "clientInstanceId", "client_instance_id") == expected]
            if not found:
                fail(f"expected worker clientInstanceId={expected!r}")
            if not any(str(get_any(w, "status", default="")).lower() == "online" for w in found):
                fail(f"expected worker clientInstanceId={expected!r} to be online")

    by_domain: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for worker in workers:
        master = get_any(worker, "master", default={}) or {}
        domain = get_any(master, "domain", default=None) or get_any(worker, "workerDomain", "worker_domain", default="unknown")
        by_domain[str(domain)].append(worker)
    for domain, group in by_domain.items():
        online_group = [w for w in group if str(get_any(w, "status", default="")).lower() == "online"]
        if not online_group:
            continue
        masters = [w for w in online_group if bool(get_any(get_any(w, "master", default={}) or {}, "isMaster", "is_master", default=False))]
        if len(masters) != 1:
            fail(f"domain {domain!r} expected exactly one master among online workers, got {len(masters)}")
        master_id = get_any(masters[0], "workerId", "worker_id")
        for worker in online_group:
            summary = get_any(worker, "master", default={}) or {}
            advertised = get_any(summary, "masterWorkerId", "master_worker_id", default=master_id)
            if advertised and advertised != master_id:
                fail(f"domain {domain!r} worker {get_any(worker, 'workerId', 'worker_id')} advertises master {advertised}, expected {master_id}")
            if get_any(summary, "term", default=1) in (None, ""):
                fail(f"domain {domain!r} worker {get_any(worker, 'workerId', 'worker_id')} missing master term")
            if get_any(summary, "fencingToken", "fencing_token", default="token") in (None, ""):
                fail(f"domain {domain!r} worker {get_any(worker, 'workerId', 'worker_id')} missing fencing token")

    blob = text_blob(workers)
    for capability in args.require_capability or []:
        if capability not in blob:
            fail(f"expected capability/tag text {capability!r} in workers payload")
    for processor in args.require_sdk_processor or []:
        if not any(processor in (get_any(get_any(w, "structuredCapabilities", "structured_capabilities", default={}) or {}, "sdkProcessors", "sdk_processors", default=[]) or []) for w in workers):
            fail(f"expected sdk processor {processor!r} in structuredCapabilities.sdkProcessors")
    for expected in args.require_plugin_processor or []:
        if ":" not in expected:
            fail(f"plugin processor expectation must be type:name, got {expected!r}")
        plugin_type, processor_name = expected.split(":", 1)
        matched = False
        for worker in workers:
            structured = get_any(worker, "structuredCapabilities", "structured_capabilities", default={}) or {}
            for plugin in get_any(structured, "pluginProcessors", "plugin_processors", default=[]) or []:
                if get_any(plugin, "type", "r#type") == plugin_type and processor_name in (get_any(plugin, "processorNames", "processor_names", default=[]) or []):
                    matched = True
        if not matched:
            fail(f"expected plugin processor {plugin_type}:{processor_name}")
    for language in args.require_script_runner or []:
        matched = False
        for worker in workers:
            structured = get_any(worker, "structuredCapabilities", "structured_capabilities", default={}) or {}
            for runner in get_any(structured, "scriptRunners", "script_runners", default=[]) or []:
                if get_any(runner, "language") == language:
                    matched = True
        if not matched:
            fail(f"expected script runner language {language!r}")
    return {"workers": len(workers), "domains": len(by_domain), "status": "passed"}


def assert_instance(args: argparse.Namespace) -> dict[str, Any]:
    inst = data(load_json(args.file))
    if not isinstance(inst, dict):
        fail("expected instance JSON data object")
    status = get_any(inst, "status")
    if args.expected_status and status != args.expected_status:
        fail(f"expected instance status {args.expected_status!r}, got {status!r}")
    worker = get_any(inst, "workerId", "worker_id")
    if args.expected_worker and worker != args.expected_worker:
        fail(f"expected instance worker {args.expected_worker!r}, got {worker!r}")
    if args.require_worker and not worker:
        fail("expected instance workerId to be present")
    if args.min_log_count is not None:
        count = get_any(inst, "logCount", "log_count", default=0) or 0
        if int(count) < args.min_log_count:
            fail(f"expected logCount >= {args.min_log_count}, got {count}")
    logs_payload = load_json(args.logs_file) if args.logs_file else None
    if args.require_log_text:
        if logs_payload is None:
            fail("--require-log-text requires --logs-file")
        blob = text_blob(logs_payload)
        for expected in args.require_log_text:
            if expected not in blob:
                fail(f"expected log text {expected!r}")
    if args.forbid_duplicate_logs and logs_payload is not None:
        log_items = items(logs_payload)
        seen: set[tuple[str, str]] = set()
        for log in log_items:
            key = (str(get_any(log, "workerId", "worker_id", default="")), str(get_any(log, "message", default="")))
            if key in seen:
                fail(f"duplicate log message detected for worker={key[0]!r}: {key[1]!r}")
            seen.add(key)
    return {"instance": get_any(inst, "id"), "status": status, "workerId": worker}


def assert_attempts(args: argparse.Namespace) -> dict[str, Any]:
    attempt_items = items(load_json(args.file))
    if len(attempt_items) < args.min_attempts:
        fail(f"expected at least {args.min_attempts} attempts, got {len(attempt_items)}")
    if args.expected_status:
        bad = [a for a in attempt_items if get_any(a, "status") != args.expected_status]
        if bad:
            fail(f"expected all attempts status {args.expected_status!r}, got {[get_any(a, 'status') for a in bad]}")
    worker_ids = {get_any(a, "workerId", "worker_id") for a in attempt_items}
    for worker_id in args.require_worker or []:
        if worker_id not in worker_ids:
            fail(f"expected broadcast attempt for worker {worker_id!r}")
    return {"attempts": len(attempt_items), "workers": sorted(worker_ids)}


def assert_web(args: argparse.Namespace) -> dict[str, Any]:
    text = Path(args.file).read_text(encoding="utf-8", errors="replace")
    for expected in args.require_text or []:
        if expected not in text:
            fail(f"expected text {expected!r} in {args.file}")
    for forbidden in args.forbid_text or []:
        if forbidden in text:
            fail(f"forbidden text {forbidden!r} found in {args.file}")
    return {"file": args.file, "status": "passed"}


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="kind", required=True)

    workers = sub.add_parser("workers")
    workers.add_argument("file")
    workers.add_argument("--min-online", type=int, default=None)
    workers.add_argument("--client-instance", action="append")
    workers.add_argument("--require-capability", action="append")
    workers.add_argument("--require-sdk-processor", action="append")
    workers.add_argument("--require-plugin-processor", action="append")
    workers.add_argument("--require-script-runner", action="append")
    workers.set_defaults(func=assert_workers)

    instance = sub.add_parser("instance")
    instance.add_argument("file")
    instance.add_argument("--expected-status")
    instance.add_argument("--expected-worker")
    instance.add_argument("--require-worker", action="store_true")
    instance.add_argument("--min-log-count", type=int, default=None)
    instance.add_argument("--logs-file")
    instance.add_argument("--require-log-text", action="append")
    instance.add_argument("--forbid-duplicate-logs", action="store_true")
    instance.set_defaults(func=assert_instance)

    attempts = sub.add_parser("attempts")
    attempts.add_argument("file")
    attempts.add_argument("--min-attempts", type=int, default=1)
    attempts.add_argument("--expected-status")
    attempts.add_argument("--require-worker", action="append")
    attempts.set_defaults(func=assert_attempts)

    web = sub.add_parser("web")
    web.add_argument("file")
    web.add_argument("--require-text", action="append")
    web.add_argument("--forbid-text", action="append")
    web.set_defaults(func=assert_web)
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        result = args.func(args)
    except AssertionFailure as error:
        print(f"ASSERTION FAILED: {error}", file=sys.stderr)
        return 1
    print(json.dumps({"status": "passed", "kind": args.kind, "result": result}, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
