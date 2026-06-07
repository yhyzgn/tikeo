#!/usr/bin/env python3
"""Synchronize release versions inside the GitHub Actions workspace.

This script intentionally edits files in-place without committing them. Release workflows run it
immediately after resolving a tag so package manifests, generated archives, and registry uploads use
the same version as the release tag.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:[A-Za-z0-9.+-]+)?$")


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write(path: Path, text: str, *, dry_run: bool) -> None:
    if dry_run:
        return
    path.write_text(text, encoding="utf-8")


def replace_once(path: Path, pattern: str, replacement: str, *, dry_run: bool, label: str) -> None:
    text = read(path)
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.MULTILINE)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match in {path}, found {count}")
    write(path, updated, dry_run=dry_run)
    print(f"set {label}: {path.relative_to(ROOT)}")


def set_json_version(path: Path, version: str, *, dry_run: bool, label: str) -> None:
    data = json.loads(read(path))
    if "version" not in data:
        raise SystemExit(f"{label}: missing version field in {path}")
    data["version"] = version
    if not dry_run:
        path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"set {label}: {path.relative_to(ROOT)}")


def sync_java_version(version: str, *, dry_run: bool) -> None:
    replace_once(
        ROOT / "sdks/java/gradle.properties",
        r"^tikeoVersion=.*$",
        f"tikeoVersion={version}",
        dry_run=dry_run,
        label="Java SDK version",
    )


def sync_rust_sdk_version(version: str, *, dry_run: bool) -> None:
    replace_once(
        ROOT / "sdks/rust/tikeo/Cargo.toml",
        r'^version = "[^"]+"$',
        f'version = "{version}"',
        dry_run=dry_run,
        label="Rust SDK version",
    )


def sync_python_version(version: str, *, dry_run: bool) -> None:
    replace_once(
        ROOT / "sdks/python/tikeo/pyproject.toml",
        r'^version = "[^"]+"$',
        f'version = "{version}"',
        dry_run=dry_run,
        label="Python SDK version",
    )


def sync_nodejs_version(version: str, *, dry_run: bool) -> None:
    set_json_version(
        ROOT / "sdks/nodejs/tikeo/package.json",
        version,
        dry_run=dry_run,
        label="Node.js SDK version",
    )


def sync_go_version() -> None:
    print("Go SDK version: tag-based module version, no package-local manifest field to update")


def sync_sdk_versions(version: str, *, dry_run: bool) -> None:
    sync_java_version(version, dry_run=dry_run)
    sync_rust_sdk_version(version, dry_run=dry_run)
    sync_python_version(version, dry_run=dry_run)
    sync_nodejs_version(version, dry_run=dry_run)
    sync_go_version()

def sync_workspace_versions(version: str, tag: str, *, dry_run: bool) -> None:
    replace_once(
        ROOT / "Cargo.toml",
        r'^version = "[^"]+"$',
        f'version = "{version}"',
        dry_run=dry_run,
        label="Rust workspace version",
    )
    replace_once(
        ROOT / "deploy/helm/tikeo/Chart.yaml",
        r"^version: .+$",
        f"version: {version}",
        dry_run=dry_run,
        label="Helm chart version",
    )
    replace_once(
        ROOT / "deploy/helm/tikeo/Chart.yaml",
        r"^appVersion: .+$",
        f'appVersion: "{version}"',
        dry_run=dry_run,
        label="Helm appVersion",
    )
    values = ROOT / "deploy/helm/tikeo/values.yaml"
    text = read(values)
    updated, count = re.subn(r"(?m)^(\s*tag: ).+$", rf"\g<1>{tag}", text)
    if count < 2:
        raise SystemExit(f"Helm values image tags: expected at least two tag fields in {values}, found {count}")
    write(values, updated, dry_run=dry_run)
    print(f"set Helm image tags: {values.relative_to(ROOT)} ({count} fields)")


def main() -> int:
    parser = argparse.ArgumentParser(description="Set release versions in workspace manifests")
    parser.add_argument("version", help="Release version without leading v, for example 0.1.123")
    parser.add_argument("--tag", help="Release tag, defaults to v<version>")
    parser.add_argument(
        "--scope",
        choices=["all", "sdk", "java", "rust", "go", "python", "nodejs", "workspace"],
        default="all",
        help="Manifest scope to update. Use language scopes in package workflows.",
    )
    parser.add_argument("--sdk-only", action="store_true", help="Deprecated alias for --scope sdk")
    parser.add_argument("--all", action="store_true", help="Deprecated alias for --scope all")
    parser.add_argument("--dry-run", action="store_true", help="Validate and print updates without writing files")
    args = parser.parse_args()

    version = args.version.strip()
    if version.startswith("v"):
        raise SystemExit("version must not include leading v; pass 0.1.123, not v0.1.123")
    if not VERSION_RE.match(version):
        raise SystemExit(f"unsupported release version format: {version}")
    tag = (args.tag or f"v{version}").strip()
    if not tag.startswith("v"):
        raise SystemExit(f"release tag must start with v: {tag}")
    if args.sdk_only and args.all:
        raise SystemExit("--sdk-only and --all are mutually exclusive")
    scope = args.scope
    if args.sdk_only:
        scope = "sdk"
    if args.all:
        scope = "all"

    if scope == "all":
        sync_sdk_versions(version, dry_run=args.dry_run)
        sync_workspace_versions(version, tag, dry_run=args.dry_run)
    elif scope == "sdk":
        sync_sdk_versions(version, dry_run=args.dry_run)
    elif scope == "java":
        sync_java_version(version, dry_run=args.dry_run)
    elif scope == "rust":
        sync_rust_sdk_version(version, dry_run=args.dry_run)
    elif scope == "go":
        sync_go_version()
    elif scope == "python":
        sync_python_version(version, dry_run=args.dry_run)
    elif scope == "nodejs":
        sync_nodejs_version(version, dry_run=args.dry_run)
    elif scope == "workspace":
        sync_workspace_versions(version, tag, dry_run=args.dry_run)
    else:
        raise SystemExit(f"unsupported scope: {scope}")
    mode = "dry-run" if args.dry_run else "write"
    print(f"release version sync complete: version={version} tag={tag} mode={mode}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
