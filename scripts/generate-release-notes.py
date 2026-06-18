#!/usr/bin/env python3
"""Generate product-style GitHub Release notes from commits and release assets.

The output is intentionally user-facing rather than a bilingual raw commit dump.
It keeps the process fully automatic: commit messages and changed paths are grouped
into product areas, assets are summarized into a download table, and raw commits are
kept only as a compact audit trail.
"""

from __future__ import annotations

import argparse
import re
import subprocess
from dataclasses import dataclass, field
from pathlib import Path


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
    release_value: str


AREAS: tuple[Area, ...] = (
    Area(
        "release",
        "Release experience",
        "Release pages now read more like product updates and less like raw CI output.",
        ("release notes", "generate-release-notes", "release body", "github release", "release-github-assets"),
        "Improves the generated GitHub Release page so operators can understand the version quickly.",
    ),
    Area(
        "migration",
        "Migration toolkit",
        "Legacy scheduler migration packaging and guidance improved.",
        ("crates/tikeo-migrate/", "migration", "migrate", "xxl", "powerjob", "legacy", "tikeo-migrate"),
        "Improves the path from XXL-JOB or PowerJob evidence to a reviewed Tikeo migration bundle.",
    ),
    Area(
        "server",
        "Server & scheduling",
        "Control-plane scheduling, dispatch, persistence, or cluster behavior changed.",
        ("crates/tikeo-server/", "crates/tikeo-core/", "crates/tikeo-storage/", "server", "raft", "scheduler", "dispatch", "worker tunnel"),
        "Improves scheduling, dispatch ownership, or control-plane runtime behavior.",
    ),
    Area(
        "web",
        "Web console",
        "The web console, user workflows, or UI behavior changed.",
        ("web/", "console", "ui", "notification", "drawer", "frontend"),
        "Improves day-to-day operation from the browser console.",
    ),
    Area(
        "sdk",
        "SDKs & workers",
        "Language SDKs, worker demos, or worker runtime integrations changed.",
        ("sdks/", "examples/", "sdk", "worker", "java", "python", "node", "rust", "go"),
        "Improves worker integration and multi-language runtime compatibility.",
    ),
    Area(
        "deploy",
        "Deployment & operations",
        "Deployment assets, container images, Helm, Kubernetes, Terraform, or release packaging changed.",
        ("deploy/", "docker", "helm", "k8s", "kubernetes", "terraform", ".github/workflows/"),
        "Improves the assets operators use to install, upgrade, and automate Tikeo environments.",
    ),
    Area(
        "docs",
        "Documentation",
        "README, documentation site, or operator guidance changed.",
        ("README", "docs/", "CHANGELOG", "documentation", "docs"),
        "Improves the deployment, integration, or operator guidance shipped with the project.",
    ),
    Area(
        "ci",
        "CI & quality gates",
        "Validation, smoke tests, workflow policy, or build reliability changed.",
        (".github/tests/", "scripts/", "ci", "test", "smoke", "workflow", "coverage"),
        "Improves the validation pipeline behind each published artifact.",
    ),
)

FIX_WORDS = ("fix", "fixed", "prevent", "repair", "bug", "failure", "failing", "stabilize", "correct")
ADD_WORDS = ("add", "added", "introduce", "support", "implement", "enable", "new")
CHANGE_WORDS = ("change", "changed", "update", "improve", "refine", "rewrite", "replace", "trim", "stabilize", "complete", "generate")

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

CODENAME_HINTS = (
    ("release", "Signal"),
    ("migration", "Bridge"),
    ("server", "Keystone"),
    ("deploy", "Harbor"),
    ("sdk", "Relay"),
    ("web", "Beacon"),
)

AREA_BY_KEY = {area.key: area for area in AREAS}


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

    if "generate-release-notes" in files or "release notes" in subject or "product-style release" in subject:
        return AREA_BY_KEY["release"]
    if "crates/tikeo-migrate/" in files or "migration cli" in subject or "tikeo-migrate" in subject:
        return AREA_BY_KEY["migration"]
    if "release asset" in subject or ".github/workflows/release" in files:
        return AREA_BY_KEY["deploy"]

    for area in AREAS:
        if any(pattern.lower() in haystack for pattern in area.patterns):
            return area
    return Area("project", "Project updates", "General project maintenance changed.", (), "Includes general maintenance for this release.")


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


def user_facing_change(commit: Commit) -> str:
    subject = clean_subject(commit.subject)
    area = classify_area(commit)
    lower = subject.lower()
    if area.key == "release" and "release notes" in lower:
        return "Upgrades the GitHub Release page into a product-oriented summary with highlights, downloads, upgrade notes, and commit audit."
    if area.key == "migration" and "source size" in lower:
        return "Keeps the migration CLI implementation within repository quality gates so release builds stay publishable."
    if area.key == "migration" and "sqlite" in lower:
        return "Improves cross-platform SQLite fixture handling for the migration CLI, including Windows local paths."
    if area.key == "deploy" and "release asset" in lower:
        return "Improves GitHub Release asset packaging and upload reliability."
    return subject


def bullet(commit: Commit) -> str:
    return f"- {user_facing_change(commit)} (`{commit.sha}`)"


def area_groups(commits: list[Commit]) -> list[tuple[Area, list[Commit]]]:
    grouped: dict[str, list[Commit]] = {}
    areas: dict[str, Area] = {}
    for commit in commits:
        area = classify_area(commit)
        grouped.setdefault(area.key, []).append(commit)
        areas[area.key] = area
    return [(areas[key], grouped[key]) for key in sorted(grouped, key=lambda k: (-len(grouped[k]), k))]


def top_highlights(commits: list[Commit], limit: int = 5) -> list[str]:
    highlights: list[str] = []
    for area, area_commits in area_groups(commits)[:limit]:
        examples = " ".join(user_facing_change(c).rstrip(".") + "." for c in area_commits[:2])
        highlights.append(f"- **{area.title}** — {area.release_value} {examples}".strip())
    if not highlights:
        highlights.append("- **Release refresh** — Packages the latest validated Tikeo build and distribution assets.")
    return highlights


def group_commits(commits: list[Commit]) -> dict[str, list[Commit]]:
    groups = {"added": [], "changed": [], "fixed": []}
    for commit in commits:
        groups[change_kind(commit.subject)].append(commit)
    return groups


def asset_label(name: str) -> str:
    if name.endswith("sdk-" + version_from_asset(name) + ".tar.gz") or re.match(r"^(go|java|nodejs|python|rust)-sdk-", name):
        return "SDK source package"
    if name.startswith("tikeo-server"):
        return "Server binary"
    if name.startswith("tikeo-migrate"):
        return "Migration CLI"
    if name.startswith("tikeo-web-dist"):
        return "Web console dist"
    if name.startswith("tikeo-deploy-sources"):
        return "Deployment source bundle"
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
    return "Release asset"


def version_from_asset(name: str) -> str:
    match = re.search(r"(\d+\.\d+\.\d+(?:[-+][A-Za-z0-9_.-]+)?)", name)
    return match.group(1) if match else ""


def asset_platform(name: str) -> str:
    sdk_names = {
        "go-sdk": "Go",
        "java-sdk": "Java",
        "nodejs-sdk": "Node.js",
        "python-sdk": "Python",
        "rust-sdk": "Rust",
    }
    for prefix, label in sdk_names.items():
        if name.startswith(prefix):
            return label
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


def asset_order(name: str) -> tuple[int, str]:
    label = asset_label(name)
    order = {
        "Server binary": 0,
        "Migration CLI": 1,
        "Web console dist": 2,
        "Helm chart": 3,
        "Docker Compose": 4,
        "Kubernetes manifest": 5,
        "Kubernetes operator": 6,
        "Terraform provider": 7,
        "SDK source package": 8,
        "Deployment source bundle": 9,
        "Release asset": 10,
    }
    return (order.get(label, 99), name)


def list_assets(path: Path) -> list[str]:
    if not path.exists():
        return []
    return sorted((file.name for file in path.iterdir() if file.is_file()), key=asset_order)


def render_asset_table(assets: list[str]) -> list[str]:
    if not assets:
        return ["Release assets were not generated for this run."]
    lines = ["| Package | Platform / profile | Asset |", "| --- | --- | --- |"]
    for name in assets:
        lines.append(f"| {asset_label(name)} | {asset_platform(name)} | `{name}` |")
    return lines


def codename_for(tag: str, commits: list[Commit]) -> str:
    area_keys = [area.key for area, _ in area_groups(commits)]
    for key, codename in CODENAME_HINTS:
        if key in area_keys:
            return codename
    numbers = [int(part) for part in re.findall(r"\d+", tag)]
    seed = sum(numbers) if numbers else len(tag)
    return CODENAMES[seed % len(CODENAMES)]


def one_line_summary(commits: list[Commit]) -> str:
    areas = [area.title for area, _ in area_groups(commits)]
    if not areas:
        return "This release packages the latest validated Tikeo build and distribution assets."
    if len(areas) == 1:
        return f"This release focuses on {areas[0].lower()} and ships a matching set of validated artifacts."
    if len(areas) == 2:
        return f"This release focuses on {areas[0].lower()} and {areas[1].lower()}, with a matching set of validated artifacts."
    return f"This release updates {', '.join(area.lower() for area in areas[:-1])}, and {areas[-1].lower()}, with a matching set of validated artifacts."


def render_group(title: str, commits: list[Commit], empty: str) -> list[str]:
    return [f"## {title}", "", *([bullet(c) for c in commits] or [f"- {empty}"]), ""]


def render_notes(tag: str, previous: str | None, commits: list[Commit], assets: list[str]) -> str:
    version = tag[1:] if tag.startswith("v") else tag
    range_label = f"{previous} → {tag}" if previous else f"initial history → {tag}"
    codename = codename_for(tag, commits)
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
    lines.extend(render_group("Added", groups["added"], "No new capability entries were detected in this release range."))
    lines.extend(render_group("Changed", groups["changed"], "No behavior-changing entries were detected in this release range."))
    lines.extend(render_group("Fixed", groups["fixed"], "No fix entries were detected in this release range."))
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
