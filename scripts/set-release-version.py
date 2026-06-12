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


def workspace_package_names() -> list[str]:
    try:
        import tomllib
    except ModuleNotFoundError as exc:  # pragma: no cover - CI uses Python 3.11+
        raise SystemExit("Python 3.11+ is required to parse Cargo manifests") from exc

    names: list[str] = []
    manifest_paths = [ROOT / "Cargo.toml", *sorted((ROOT / "crates").glob("*/Cargo.toml"))]
    for manifest in manifest_paths:
        data = tomllib.loads(read(manifest))
        package = data.get("package")
        if not package or "name" not in package:
            continue
        names.append(str(package["name"]))
    return names


def sync_workspace_lock_versions(version: str, *, dry_run: bool) -> None:
    lock = ROOT / "Cargo.lock"
    text = read(lock)
    names = set(workspace_package_names())
    if not names:
        raise SystemExit("Cargo.lock workspace version sync: no workspace packages found")

    package_header = "[[package]]"
    parts = text.split(package_header)
    updated_parts = [parts[0]]
    changed: list[str] = []
    for part in parts[1:]:
        block = package_header + part
        name_match = re.search(r'(?m)^name = "([^"]+)"$', block)
        if name_match and name_match.group(1) in names and "\nsource = " not in block:
            block, count = re.subn(r'(?m)^version = "[^"]+"$', f'version = "{version}"', block, count=1)
            if count != 1:
                raise SystemExit(f"Cargo.lock workspace version sync: missing version for {name_match.group(1)}")
            changed.append(name_match.group(1))
        updated_parts.append(block)

    missing = names.difference(changed)
    if missing:
        raise SystemExit(
            "Cargo.lock workspace version sync: missing workspace packages in lockfile: "
            + ", ".join(sorted(missing))
        )
    write(lock, "".join(updated_parts), dry_run=dry_run)
    print(f"set Rust workspace lock versions: {lock.relative_to(ROOT)} ({len(changed)} packages)")


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
    sync_workspace_lock_versions(version, dry_run=dry_run)
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
