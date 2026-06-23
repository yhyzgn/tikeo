#!/usr/bin/env python3
"""Fail if a release tag does not match committed manifest versions."""

from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path

LIB_PATH = Path(__file__).with_name("release-version-lib.py")
import sys
spec = importlib.util.spec_from_file_location("release_version_lib", LIB_PATH)
lib = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = lib
spec.loader.exec_module(lib)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate manifest-first release version consistency")
    parser.add_argument("version", nargs="?", help="Version without leading v. Defaults to --tag without v.")
    parser.add_argument("--tag", required=True, help="Release tag, for example v0.3.11")
    args = parser.parse_args()

    version = lib.normalize_version(args.version or args.tag)
    tag = args.tag.strip()
    issues = lib.check_release_versions(version, tag=tag)
    if issues:
        print(f"release version check failed for version={version} tag={tag}")
        for issue in issues:
            print(f"- {issue.label}: expected {issue.expected}, actual {issue.actual}")
        print(f"Run: python scripts/prepare-release.py {version}")
        return 1
    print(f"release version check passed: version={version} tag={tag}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
