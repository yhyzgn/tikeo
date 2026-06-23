#!/usr/bin/env python3
"""Prepare a manifest-first Tikeo release commit."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    parser = argparse.ArgumentParser(description="Update committed manifests for a Tikeo release")
    parser.add_argument("version", help="Release version without leading v, for example 0.3.11")
    parser.add_argument("--tag", help="Release tag, defaults to v<version>")
    parser.add_argument("--dry-run", action="store_true", help="Print intended changes without writing")
    args = parser.parse_args()

    tag = args.tag or f"v{args.version}"
    cmd = ["python3", "scripts/set-release-version.py", args.version, "--tag", tag, "--scope", "workspace"]
    if args.dry_run:
        cmd.append("--dry-run")
    subprocess.run(cmd, cwd=ROOT, check=True)
    subprocess.run(["python3", "scripts/check-release-version.py", args.version, "--tag", tag], cwd=ROOT, check=True)
    print("release commit is ready after review:")
    print(f"  git add Cargo.toml Cargo.lock web/package.json docs/package.json deploy/helm/tikeo/Chart.yaml deploy/helm/tikeo/values.yaml README.md README.zh-CN.md deploy/helm/tikeo/README.md docs/docs/deployment/production.md docs/i18n/zh-CN/docusaurus-plugin-content-docs/current/deployment/production.md docs/docs/reference/configuration-cookbook.md docs/i18n/zh-CN/docusaurus-plugin-content-docs/current/reference/configuration-cookbook.md")
    print(f"  git commit -m 'chore(release): {tag}'")
    print(f"  git tag -a {tag} -m '{tag}'")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
