#!/usr/bin/env python3
"""Shared release version manifest helpers for Tikeo."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:[A-Za-z0-9.+-]+)?$")


@dataclass(frozen=True)
class VersionIssue:
    path: str
    expected: str
    actual: str
    label: str


def normalize_version(version: str) -> str:
    value = version.strip()
    if value.startswith("v"):
        value = value[1:]
    if not VERSION_RE.match(value):
        raise SystemExit(f"unsupported release version format: {version}")
    return value


def tag_for(version: str) -> str:
    return f"v{normalize_version(version)}"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def json_version(path: Path) -> str:
    return str(json.loads(read(path)).get("version", ""))


def regex_value(path: Path, pattern: str, label: str) -> str:
    match = re.search(pattern, read(path), flags=re.MULTILINE)
    if not match:
        raise SystemExit(f"{label}: no match in {path}")
    return match.group(1)


def workspace_package_names() -> list[str]:
    try:
        import tomllib
    except ModuleNotFoundError as exc:  # pragma: no cover
        raise SystemExit("Python 3.11+ is required to parse Cargo manifests") from exc

    names: list[str] = []
    manifest_paths = [ROOT / "Cargo.toml", *sorted((ROOT / "crates").glob("*/Cargo.toml"))]
    for manifest in manifest_paths:
        data = tomllib.loads(read(manifest))
        package = data.get("package")
        if package and package.get("name"):
            names.append(str(package["name"]))
    return names


def cargo_lock_workspace_versions() -> dict[str, str]:
    versions: dict[str, str] = {}
    names = set(workspace_package_names())
    for raw_block in read(ROOT / "Cargo.lock").split("[[package]]"):
        block = raw_block.strip()
        if not block or "\nsource = " in f"\n{block}\n":
            continue
        name_match = re.search(r'(?m)^name = "([^"]+)"$', block)
        version_match = re.search(r'(?m)^version = "([^"]+)"$', block)
        if name_match and version_match and name_match.group(1) in names:
            versions[name_match.group(1)] = version_match.group(1)
    return versions


def manifest_versions() -> dict[str, str]:
    versions = {
        "Cargo.toml workspace.package.version": regex_value(ROOT / "Cargo.toml", r'^version = "([^"]+)"$', "Rust workspace version"),
        "web/package.json version": json_version(ROOT / "web/package.json"),
        "docs/package.json version": json_version(ROOT / "docs/package.json"),
        "deploy/helm/tikeo/Chart.yaml version": regex_value(ROOT / "deploy/helm/tikeo/Chart.yaml", r"^version: (.+)$", "Helm chart version").strip().strip('"'),
        "deploy/helm/tikeo/Chart.yaml appVersion": regex_value(ROOT / "deploy/helm/tikeo/Chart.yaml", r"^appVersion: (.+)$", "Helm appVersion").strip().strip('"'),
    }
    for name, value in cargo_lock_workspace_versions().items():
        versions[f"Cargo.lock {name}"] = value
    return versions


def check_release_versions(version: str, *, tag: str | None = None) -> list[VersionIssue]:
    expected = normalize_version(version)
    expected_tag = tag or tag_for(expected)
    if expected_tag != tag_for(expected):
        return [VersionIssue("<tag>", tag_for(expected), expected_tag, "git tag must equal v<version>")]

    issues: list[VersionIssue] = []
    for label, actual in manifest_versions().items():
        if actual != expected:
            issues.append(VersionIssue(label.split()[0], expected, actual, label))
    return issues
