#!/usr/bin/env python3
"""Reject GitHub Actions JavaScript actions that target deprecated Node runtimes.

This script scans workflow files for external ``uses: owner/repo[/path]@ref`` steps,
fetches each action's metadata from GitHub, and fails when ``runs.using`` is a
JavaScript runtime below the configured minimum (Node 24 by default in CI).
It is intentionally stdlib-only so it can run at the start of CI before language
or package-manager bootstrapping.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

USES_RE = re.compile(r"^\s*-?\s*uses\s*:\s*['\"]?([^'\"\s#]+)['\"]?\s*(?:#.*)?$")
USING_RE = re.compile(r"(?im)^\s*using\s*:\s*['\"]?([^'\"#\s]+)['\"]?")
NODE_RUNTIME_RE = re.compile(r"^node(\d+)$", re.IGNORECASE)


@dataclass(frozen=True)
class ActionUse:
    spec: str
    workflow: Path
    line: int


@dataclass(frozen=True)
class ActionMetadata:
    spec: str
    using: str
    source: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--workflows-dir",
        default=".github/workflows",
        type=Path,
        help="Directory containing GitHub Actions workflow YAML files.",
    )
    parser.add_argument(
        "--min-node-major",
        default=24,
        type=int,
        help="Minimum accepted node runtime major for JavaScript actions.",
    )
    parser.add_argument(
        "--timeout-seconds",
        default=20,
        type=int,
        help="HTTP timeout per metadata request.",
    )
    return parser.parse_args()


def iter_workflow_files(workflows_dir: Path) -> Iterable[Path]:
    if not workflows_dir.exists():
        raise FileNotFoundError(f"workflow directory not found: {workflows_dir}")
    for suffix in ("*.yml", "*.yaml"):
        yield from sorted(workflows_dir.glob(suffix))


def iter_action_uses(workflows_dir: Path) -> Iterable[ActionUse]:
    for workflow in iter_workflow_files(workflows_dir):
        for line_no, line in enumerate(workflow.read_text(encoding="utf-8").splitlines(), start=1):
            match = USES_RE.match(line)
            if match:
                yield ActionUse(match.group(1), workflow, line_no)


def is_external_github_action(spec: str) -> bool:
    return not (
        spec.startswith("./")
        or spec.startswith("../")
        or spec.startswith("docker://")
        or spec.startswith("http://")
        or spec.startswith("https://")
    )


def split_action_spec(spec: str) -> tuple[str, str, str, str]:
    if "@" not in spec:
        raise ValueError(f"external action is not pinned with @ref: {spec}")
    action_path, ref = spec.rsplit("@", 1)
    parts = action_path.split("/")
    if len(parts) < 2 or not parts[0] or not parts[1] or not ref:
        raise ValueError(f"invalid GitHub action reference: {spec}")
    owner, repo = parts[0], parts[1]
    subpath = "/".join(parts[2:])
    return owner, repo, subpath, ref


def github_raw_request(url: str, timeout: int) -> bytes:
    headers = {"User-Agent": "tikee-node-runtime-policy-check"}
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return response.read()


def github_api_request(url: str, timeout: int) -> bytes:
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "tikee-node-runtime-policy-check",
    }
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return response.read()


def fetch_action_file_from_raw(owner: str, repo: str, path: str, ref: str, timeout: int) -> str | None:
    url = f"https://raw.githubusercontent.com/{owner}/{repo}/{urllib.parse.quote(ref, safe='')}/{path}"
    last_error: Exception | None = None
    for attempt in range(3):
        try:
            return github_raw_request(url, timeout).decode("utf-8")
        except urllib.error.HTTPError as exc:
            if exc.code == 404:
                return None
            last_error = exc
        except (urllib.error.URLError, OSError) as exc:
            last_error = exc
        if attempt < 2:
            time.sleep(0.35 * (attempt + 1))
    if isinstance(last_error, urllib.error.HTTPError):
        raise RuntimeError(f"raw metadata fetch failed with HTTP {last_error.code}") from last_error
    raise RuntimeError(f"raw metadata fetch failed: {last_error}") from last_error


def fetch_action_file_from_api(owner: str, repo: str, path: str, ref: str, timeout: int) -> str | None:
    url = f"https://api.github.com/repos/{owner}/{repo}/contents/{path}?ref={urllib.parse.quote(ref, safe='')}"
    try:
        payload = json.loads(github_api_request(url, timeout).decode("utf-8"))
    except urllib.error.HTTPError as exc:
        if exc.code == 404:
            return None
        raise RuntimeError(f"contents API metadata fetch failed with HTTP {exc.code}") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"contents API metadata fetch failed: {exc.reason}") from exc
    content = payload.get("content")
    encoding = payload.get("encoding")
    if encoding != "base64" or not isinstance(content, str):
        raise RuntimeError(f"unexpected GitHub contents response for {owner}/{repo}/{path}@{ref}")
    return base64.b64decode(content).decode("utf-8")


def fetch_action_file(owner: str, repo: str, subpath: str, ref: str, filename: str, timeout: int) -> str | None:
    path = f"{subpath}/{filename}" if subpath else filename
    raw_error: RuntimeError | None = None
    try:
        return fetch_action_file_from_raw(owner, repo, path, ref, timeout)
    except RuntimeError as exc:
        raw_error = exc

    try:
        return fetch_action_file_from_api(owner, repo, path, ref, timeout)
    except RuntimeError as exc:
        raise RuntimeError(f"failed to fetch {owner}/{repo}/{path}@{ref}: {raw_error}; {exc}") from exc


def fetch_action_metadata(spec: str, timeout: int) -> ActionMetadata:
    owner, repo, subpath, ref = split_action_spec(spec)
    for filename in ("action.yml", "action.yaml"):
        content = fetch_action_file(owner, repo, subpath, ref, filename, timeout)
        if content is None:
            continue
        match = USING_RE.search(content)
        if not match:
            raise RuntimeError(f"{spec} metadata file {filename} does not declare runs.using")
        return ActionMetadata(spec=spec, using=match.group(1).lower(), source=f"{owner}/{repo}/{subpath}/{filename}@{ref}".replace("//", "/"))
    raise RuntimeError(f"{spec} has no action.yml or action.yaml metadata at the referenced ref")


def main() -> int:
    args = parse_args()
    all_uses = list(iter_action_uses(args.workflows_dir))
    external_uses = [use for use in all_uses if is_external_github_action(use.spec)]
    unique_specs = sorted({use.spec for use in external_uses})
    locations_by_spec: dict[str, list[ActionUse]] = {spec: [] for spec in unique_specs}
    for use in external_uses:
        locations_by_spec[use.spec].append(use)

    failures: list[str] = []
    metadata_by_spec: dict[str, ActionMetadata] = {}
    for spec in unique_specs:
        try:
            metadata_by_spec[spec] = fetch_action_metadata(spec, args.timeout_seconds)
        except Exception as exc:  # noqa: BLE001 - policy check should report all unresolved refs clearly.
            failures.append(f"{spec}: metadata lookup failed: {exc}")

    for spec, metadata in sorted(metadata_by_spec.items()):
        node_match = NODE_RUNTIME_RE.match(metadata.using)
        if not node_match:
            continue
        major = int(node_match.group(1))
        if major < args.min_node_major:
            locations = ", ".join(f"{use.workflow}:{use.line}" for use in locations_by_spec[spec])
            failures.append(
                f"{spec}: uses deprecated {metadata.using}; require node{args.min_node_major}+ ({locations})"
            )

    if failures:
        print("Deprecated or unresolved GitHub Actions runtimes found:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    print(f"Checked {len(unique_specs)} external GitHub actions; no Node runtime below node{args.min_node_major}.")
    for spec, metadata in sorted(metadata_by_spec.items()):
        print(f"- {spec}: {metadata.using}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
