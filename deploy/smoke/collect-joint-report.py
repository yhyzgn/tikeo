#!/usr/bin/env python3
"""Collect tikeo smoke JSON/JSONL artifacts into one reviewable report."""
from __future__ import annotations

import argparse
import datetime as dt
import json
from pathlib import Path
from typing import Any


def load_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as fh:
        return json.load(fh)


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open(encoding="utf-8") as fh:
        for line in fh:
            if line.strip():
                rows.append(json.loads(line))
    return rows


def discover_cases(report_dir: Path) -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    for path in sorted(report_dir.glob("*.jsonl")):
        if path.name.endswith("-cases.jsonl"):
            for row in load_jsonl(path):
                row.setdefault("source", str(path))
                cases.append(row)
    for path in sorted(report_dir.glob("*.json")):
        try:
            payload = load_json(path)
        except Exception:
            continue
        for key in ("functional_cases", "cases"):
            for row in payload.get(key, []) if isinstance(payload, dict) else []:
                if isinstance(row, dict):
                    normalized = dict(row)
                    normalized.setdefault("id", normalized.get("name", path.stem))
                    normalized.setdefault("source", str(path))
                    cases.append(normalized)
    unique: dict[tuple[str, str], dict[str, Any]] = {}
    for case in cases:
        key = (str(case.get("id") or case.get("name") or "unknown"), str(case.get("source", "")))
        unique[key] = case
    return list(unique.values())


def status_of(cases: list[dict[str, Any]]) -> str:
    if not cases:
        return "failed"
    statuses = {str(case.get("status", "")).lower() for case in cases}
    if any(status in {"failed", "failure", "error", "blocked"} for status in statuses):
        return "failed"
    if all(status in {"passed", "pass", "ok", "通过"} for status in statuses):
        return "passed"
    return "partial"


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# Tikeo joint automation report",
        "",
        f"Run ID: `{report['run_id']}`",
        f"Generated at: `{report['generated_at']}`",
        f"Status: **{report['status']}**",
        "",
        "| ID | 状态 | 证据 | 摘要 |",
        "| --- | --- | --- | --- |",
    ]
    for case in report["cases"]:
        case_id = case.get("id") or case.get("name") or "unknown"
        status = case.get("status", "unknown")
        evidence = case.get("evidence") or case.get("source") or "-"
        message = case.get("message") or case.get("url") or "-"
        lines.append(f"| `{case_id}` | {status} | `{evidence}` | {message} |")
    lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report_dir", type=Path)
    parser.add_argument("--run-id", default=None)
    parser.add_argument("--json-output", type=Path, default=None)
    parser.add_argument("--markdown-output", type=Path, default=None)
    args = parser.parse_args()

    report_dir = args.report_dir
    cases = discover_cases(report_dir)
    run_id = args.run_id or report_dir.name
    report = {
        "run_id": run_id,
        "generated_at": dt.datetime.now(dt.UTC).isoformat(),
        "status": status_of(cases),
        "case_count": len(cases),
        "cases": cases,
    }
    json_output = args.json_output or report_dir / "joint-report.json"
    md_output = args.markdown_output or report_dir / "joint-report.md"
    json_output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    write_markdown(md_output, report)
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
