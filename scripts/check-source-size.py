#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
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
    parser.add_argument(
        '--baseline',
        default='scripts/source-size-baseline.json',
        help='JSON map of existing over-limit files to their maximum allowed line count',
    )
    args = parser.parse_args()

    root = Path(args.root).resolve()
    baseline_path = (root / args.baseline).resolve()
    baseline: dict[str, int] = {}
    if baseline_path.exists():
        baseline = json.loads(baseline_path.read_text(encoding='utf-8'))

    offenders: list[tuple[int, Path, int]] = []
    for path in iter_sources(root):
        with path.open('r', encoding='utf-8') as handle:
            count = sum(1 for _ in handle)
        relative = path.relative_to(root)
        allowed = max(args.limit, baseline.get(relative.as_posix(), args.limit))
        if count > allowed:
            offenders.append((count, relative, allowed))

    if offenders:
        print(f'Source files exceed size budget. New files must stay <= {args.limit} lines; baseline files must not grow beyond their recorded budget:')
        for count, path, allowed in sorted(offenders, reverse=True):
            print(f'  {count:5d}/{allowed:<5d}  {path}')
        return 1

    if baseline:
        print(f'All source files are within the {args.limit}-line budget or recorded baseline.')
    else:
        print(f'All source files are <= {args.limit} lines.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
