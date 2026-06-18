#!/usr/bin/env python3
"""Generate product-style GitHub Release notes from commits and release assets.

The output is intentionally user-facing rather than a bilingual raw commit dump.
It keeps the process fully automatic: commit messages and changed paths are grouped
into product areas, assets are summarized into a download table, and raw commits are
kept only as a compact audit trail.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable


@dataclass
class Commit:
    sha: str
    date: str
    subject: str
    files: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class Area:
    key: str
    title: str
    summary: str
    patterns: tuple[str, ...]


AREAS: tuple[Area, ...] = (
    Area(
        "server",
        "Server & scheduling",
        "Control-plane scheduling, dispatch, persistence, or cluster behavior changed.",
        ("crates/tikeo-server/", "crates/tikeo-core/", "crates/tikeo-storage/", "server", "raft", "scheduler", "dispatch", "worker tunnel"),
    ),
    Area(
        "migration",
        "Migration toolkit",
        "The legacy scheduler migration workflow or migration CLI changed.",
        ("crates/tikeo-migrate/", "migration", "migrate", "xxl", "powerjob", "legacy"),
    ),
    Area(
        "web",
        "Web console",
        "The web console, user workflows, or UI behavior changed.",
        ("web/", "console", "ui", "notification", "drawer", "frontend"),
    ),
    Area(
        "sdk",
        "SDKs & workers",
        "Language SDKs, worker demos, or worker runtime integrations changed.",
        ("sdks/", "examples/", "sdk", "worker", "java", "python", "node", "rust", "go"),
    ),
    Area(
        "deploy",
        "Deployment & operations",
        "Deployment assets, container images, Helm, Kubernetes, Terraform, or release packaging changed.",
        ("deploy/", "docker", "helm", "k8s", "kubernetes", "terraform", "release", ".github/workflows/"),
    ),
    Area(
        "docs",
        "Documentation",
        "README, documentation site, or operator guidance changed.",
        ("README", "docs/", "CHANGELOG", "documentation", "docs"),
    ),
    Area(
        "ci",
        "CI & quality gates",
        "Validation, smoke tests, workflow policy, or build reliability changed.",
        (".github/tests/", "scripts/", "ci", "test", "smoke", "workflow", "coverage"),
    ),
)

FIX_WORDS = ("fix", "fixed", "prevent", "repair", "bug", "failure", "failing", "stabilize", "correct")
ADD_WORDS = ("add", "added", "introduce", "support", "implement", "enable", "new")
CHANGE_WORDS = ("change", "changed", "update", "improve", "refine", "rewrite", "replace", "trim", "stabilize", "complete")


CODENAMES = (
    "Beacon",
    "Bridge",
    "Harbor",
    "Compass",
    "Relay",
    "Foundry",
    "Atlas",
    "Keystone",
    "Signal",
    "Canopy",
)


def run_git(args: list[str]) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def previous_tag(current: str) -> str | None:
    tags = run_git(["tag", "--list", "v*", "--sort=-v:refname"]).splitlines()
    for index, tag in enumerate(tags):
        if tag == current and index + 1 < len(tags):
            return tags[index + 1]
    return None


def load_commits(release_tag: str, previous: str | None) -> list[Commit]:
    commit_range = f"{previous}..{release_tag}" if previous else release_tag
    raw = run_git(["log", "--date=short", "--pretty=format:%H%x1f%h%x1f%ad%x1f%s", commit_range])
    commits: list[Commit] = []
    for line in raw.splitlines():
        full_sha, short_sha, date, subject = line.split("\x1f", 3)
        files_raw = run_git(["diff-tree", "--no-commit-id", "--name-only", "-r", full_sha])
        commits.append(Commit(short_sha, date, subject, [f for f in files_raw.splitlines() if f]))
    return commits


def classify_area(commit: Commit) -> Area:
    subject = commit.subject.lower()
    files = " ".join(commit.files).lower()
    haystack = f"{subject} {files}"

    if "crates/tikeo-migrate/" in files or "migration cli" in subject or "tikeo-migrate" in subject:
        return next(area for area in AREAS if area.key == "migration")
    if "release asset" in subject or ".github/workflows/release" in files:
        return next(area for area in AREAS if area.key == "deploy")

    for area in AREAS:
        if any(pattern.lower() in haystack for pattern in area.patterns):
            return area
    return Area("project", "Project updates", "General project maintenance changed.", ())


def change_kind(subject: str) -> str:
    text = subject.lower()
    if any(word in text for word in FIX_WORDS):
        return "fixed"
    if any(word in text for word in ADD_WORDS):
        return "added"
    if any(word in text for word in CHANGE_WORDS):
        return "changed"
    return "changed"


def clean_subject(subject: str) -> str:
    subject = re.sub(r"^(feat|fix|docs|chore|ci|refactor|test|build)(\([^)]+\))?:\s*", "", subject, flags=re.I)
    return subject[:1].upper() + subject[1:]


def bullet(commit: Commit) -> str:
    return f"- {clean_subject(commit.subject)} (`{commit.sha}`)"


def top_highlights(commits: list[Commit], limit: int = 5) -> list[str]:
    area_to_commits: dict[str, list[Commit]] = {}
    area_by_key: dict[str, Area] = {}
    for commit in commits:
        area = classify_area(commit)
        area_to_commits.setdefault(area.key, []).append(commit)
        area_by_key[area.key] = area
    ordered = sorted(area_to_commits.items(), key=lambda item: len(item[1]), reverse=True)
    highlights: list[str] = []
    for key, area_commits in ordered[:limit]:
        area = area_by_key[key]
        examples = "; ".join(clean_subject(c.subject) for c in area_commits[:2])
        highlights.append(f"- **{area.title}** — {area.summary} Key changes: {examples}.")
    if not highlights:
        highlights.append("- **Release refresh** — This version packages the latest validated Tikeo build and release assets.")
    return highlights


def group_commits(commits: list[Commit]) -> dict[str, list[Commit]]:
    groups = {"added": [], "changed": [], "fixed": []}
    for commit in commits:
        groups[change_kind(commit.subject)].append(commit)
    return groups


def asset_label(name: str) -> str:
    if name.startswith("tikeo-server"):
        return "Server binary"
    if name.startswith("tikeo-migrate"):
        return "Migration CLI"
    if name.startswith("tikeo-web-dist"):
        return "Web console dist"
    if name.startswith("tikeo-") and name.endswith(".tgz"):
        return "Helm chart"
    if "docker-compose" in name:
        return "Docker Compose"
    if "k8s" in name or "crd" in name:
        return "Kubernetes manifest"
    if name.startswith("terraform-provider"):
        return "Terraform provider"
    if name.startswith("tikeo-operator"):
        return "Kubernetes operator"
    if name.endswith("sdk.tar.gz") or "-sdk-" in name:
        return "SDK source package"
    return "Release asset"


def asset_platform(name: str) -> str:
    platform_tokens = {
        "x86_64-unknown-linux-gnu": "Linux x86_64",
        "x86_64-pc-windows-msvc": "Windows x86_64",
        "x86_64-apple-darwin": "macOS Intel",
        "aarch64-apple-darwin": "macOS Apple Silicon",
        "linux_amd64": "Linux amd64",
        "linux_arm64": "Linux arm64",
        "darwin_amd64": "macOS Intel",
        "darwin_arm64": "macOS Apple Silicon",
        "windows_amd64": "Windows amd64",
        "linux-amd64": "Linux amd64",
        "linux-arm64": "Linux arm64",
        "darwin-amd64": "macOS Intel",
        "darwin-arm64": "macOS Apple Silicon",
        "windows-amd64": "Windows amd64",
        "sqlite": "SQLite",
        "postgres": "PostgreSQL",
        "mysql": "MySQL",
    }
    for token, label in platform_tokens.items():
        if token in name:
            return label
    return "All platforms"


def list_assets(path: Path) -> list[str]:
    if not path.exists():
        return []
    return sorted(file.name for file in path.iterdir() if file.is_file())


def render_asset_table(assets: list[str]) -> list[str]:
    if not assets:
        return ["No release assets were generated."]
    lines = ["| Package | Platform / profile | Asset |", "| --- | --- | --- |"]
    for name in assets:
        lines.append(f"| {asset_label(name)} | {asset_platform(name)} | `{name}` |")
    return lines


def codename_for(tag: str) -> str:
    numbers = [int(part) for part in re.findall(r"\d+", tag)]
    seed = sum(numbers) if numbers else len(tag)
    return CODENAMES[seed % len(CODENAMES)]


def one_line_summary(commits: list[Commit]) -> str:
    areas = []
    for commit in commits:
        title = classify_area(commit).title
        if title not in areas:
            areas.append(title)
    if not areas:
        return "This release packages the latest validated Tikeo build and distribution assets."
    if len(areas) == 1:
        return f"This release focuses on {areas[0].lower()} with validated distribution assets."
    return f"This release updates {', '.join(area.lower() for area in areas[:-1])}, and {areas[-1].lower()} with validated distribution assets."


def render_notes(tag: str, previous: str | None, commits: list[Commit], assets: list[str]) -> str:
    version = tag[1:] if tag.startswith("v") else tag
    range_label = f"{previous} → {tag}" if previous else f"initial history → {tag}"
    codename = codename_for(tag)
    groups = group_commits(commits)

    lines: list[str] = []
    lines.append(f"# Tikeo {version} — {codename}")
    lines.append("")
    lines.append(f"### Codename: {codename}")
    lines.append("")
    lines.append(one_line_summary(commits))
    lines.append("")
    lines.append("## Highlights")
    lines.append("")
    lines.extend(top_highlights(commits))
    lines.append("")
    lines.append("> Download the matching binary, SDK package, deployment manifest, or Helm chart from the assets attached below.")
    lines.append("")
    lines.append("## Downloads")
    lines.append("")
    lines.extend(render_asset_table(assets))
    lines.append("")
    lines.append("## Added")
    lines.append("")
    lines.extend([bullet(c) for c in groups["added"]] or ["- No new user-facing additions were detected in the commit range."])
    lines.append("")
    lines.append("## Changed")
    lines.append("")
    lines.extend([bullet(c) for c in groups["changed"]] or ["- No notable behavior changes were detected in the commit range."])
    lines.append("")
    lines.append("## Fixed")
    lines.append("")
    lines.extend([bullet(c) for c in groups["fixed"]] or ["- No fixes were detected in the commit range."])
    lines.append("")
    lines.append("## Upgrade notes")
    lines.append("")
    lines.append(f"- Release range: `{range_label}`.")
    lines.append("- Use assets from this tag together; mixing server, SDK, and deployment assets from different tags is not recommended.")
    if any(name.startswith("tikeo-migrate") for name in assets):
        lines.append("- Migration CLI users should download the platform-specific `tikeo-migrate` archive from this release.")
    if any("docker-compose" in name or "k8s" in name or name.endswith(".tgz") for name in assets):
        lines.append("- Deployment users should refresh Compose, Kubernetes, Helm, or Terraform artifacts from this release before rollout.")
    lines.append("")
    lines.append("## Verification")
    lines.append("")
    lines.append("- Release assets are generated by the GitHub release workflow after server, migration CLI, web, and deploy packaging jobs pass.")
    lines.append("- SDK and container publishing workflows run independently for the same tag.")
    lines.append("")
    lines.append("## Commit audit")
    lines.append("")
    lines.extend([f"- `{c.sha}` {c.date} — {c.subject}" for c in commits] or ["- No commits found for this release range."])
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate product-style Tikeo release notes")
    parser.add_argument("--tag", required=True, help="Release tag, for example v0.3.5")
    parser.add_argument("--assets-dir", default="release-assets", help="Directory containing staged release assets")
    parser.add_argument("--output", default="release-notes.md", help="Output markdown path")
    args = parser.parse_args()

    release_tag = args.tag.strip()
    subprocess.check_call(["git", "fetch", "--force", "--tags"])
    previous = previous_tag(release_tag)
    commits = load_commits(release_tag, previous)
    assets = list_assets(Path(args.assets_dir))
    Path(args.output).write_text(render_notes(release_tag, previous, commits, assets), encoding="utf-8")
    print(f"generated {args.output} for {release_tag} with {len(commits)} commits and {len(assets)} assets")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
