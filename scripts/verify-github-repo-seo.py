#!/usr/bin/env python3
"""Verify GitHub repository SEO metadata against .github/repository-seo.json."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = ROOT / ".github/repository-seo.json"


def run_gh(args: list[str]) -> dict[str, object]:
    env = os.environ.copy()
    if not env.get("GH_TOKEN"):
        try:
            token = subprocess.check_output(["gh", "auth", "token"], text=True).strip()
        except subprocess.CalledProcessError as error:
            raise RuntimeError("GH_TOKEN is not set and gh auth token is unavailable") from error
        if token:
            env["GH_TOKEN"] = token
    with tempfile.TemporaryDirectory(prefix="tikeo-gh-state-") as state_dir:
        env.setdefault("GH_STATE_DIR", state_dir)
        output = subprocess.check_output(["gh", *args], text=True, env=env)
    return json.loads(output)


def main() -> int:
    expected = json.loads(CONTRACT.read_text())
    actual = run_gh([
        "repo",
        "view",
        expected["repository"],
        "--json",
        "description,homepageUrl,repositoryTopics,url",
    ])
    actual_topics = sorted(topic["name"] for topic in actual["repositoryTopics"])
    expected_topics = sorted(expected["topics"])
    failures: list[str] = []
    if actual["description"] != expected["description"]:
        failures.append(f"description mismatch: {actual['description']!r}")
    if actual["homepageUrl"] != expected["homepageUrl"]:
        failures.append(f"homepageUrl mismatch: {actual['homepageUrl']!r}")
    if actual_topics != expected_topics:
        failures.append(f"topics mismatch: actual={actual_topics!r} expected={expected_topics!r}")
    if failures:
        for failure in failures:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"GitHub SEO metadata matches {CONTRACT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
