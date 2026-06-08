#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

DEFAULT_LIMIT = 1500
EXTENSIONS = {'.rs', '.ts', '.tsx'}
PRUNE_DIRS = {
    '.git',
    '.dev',
    '.pytest_cache',
    'target',
    'node_modules',
    'dist',
    'coverage',
}


def should_prune(path: Path) -> bool:
    return any(part in PRUNE_DIRS for part in path.parts)


def iter_sources(root: Path):
    for path in root.rglob('*'):
        if path.is_dir() or should_prune(path):
            continue
        if path.suffix in EXTENSIONS:
            yield path


def main() -> int:
    parser = argparse.ArgumentParser(description='Fail when normal source files exceed a line limit.')
    parser.add_argument('--root', default='.', help='Repository root to scan')
    parser.add_argument('--limit', type=int, default=DEFAULT_LIMIT, help='Maximum allowed physical lines')
    args = parser.parse_args()

    root = Path(args.root).resolve()
    offenders: list[tuple[int, Path]] = []
    for path in iter_sources(root):
        with path.open('r', encoding='utf-8') as handle:
            count = sum(1 for _ in handle)
        if count > args.limit:
            offenders.append((count, path.relative_to(root)))

    if offenders:
        print(f'Source files exceed {args.limit} lines:')
        for count, path in sorted(offenders, reverse=True):
            print(f'  {count:5d}  {path}')
        return 1

    print(f'All source files are <= {args.limit} lines.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
