from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def copy_release_sync_fixture(tmp_path: Path) -> Path:
    fixture = tmp_path / "repo"
    (fixture / "scripts").mkdir(parents=True)
    (fixture / "crates").mkdir()
    (fixture / "deploy/helm/tikeo").mkdir(parents=True)
    shutil.copy2(ROOT / "scripts/set-release-version.py", fixture / "scripts/set-release-version.py")
    shutil.copy2(ROOT / "Cargo.toml", fixture / "Cargo.toml")
    shutil.copy2(ROOT / "Cargo.lock", fixture / "Cargo.lock")
    for manifest in sorted((ROOT / "crates").glob("*/Cargo.toml")):
        target = fixture / "crates" / manifest.parent.name
        target.mkdir()
        shutil.copy2(manifest, target / "Cargo.toml")
    for name in ["Chart.yaml", "values.yaml"]:
        shutil.copy2(ROOT / "deploy/helm/tikeo" / name, fixture / "deploy/helm/tikeo" / name)
    return fixture


def local_workspace_lock_versions(lock_text: str, names: set[str]) -> dict[str, str]:
    versions: dict[str, str] = {}
    for raw_block in lock_text.split("[[package]]"):
        block = raw_block.strip()
        if not block or "\nsource = " in f"\n{block}\n":
            continue
        fields = {}
        for line in block.splitlines():
            if line.startswith("name = ") or line.startswith("version = "):
                key, value = line.split(" = ", 1)
                fields[key] = value.strip('"')
        name = fields.get("name")
        if name in names:
            versions[name] = fields["version"]
    return versions


def test_workspace_release_sync_updates_cargo_lock_for_locked_release_builds(tmp_path: Path):
    fixture = copy_release_sync_fixture(tmp_path)
    subprocess.run(
        ["python3", "scripts/set-release-version.py", "0.2.9", "--tag", "v0.2.9", "--scope", "workspace"],
        cwd=fixture,
        check=True,
    )

    names = {"tikeo", *[path.parent.name.replace("_", "-") for path in (fixture / "crates").glob("*/Cargo.toml")]}
    versions = local_workspace_lock_versions((fixture / "Cargo.lock").read_text(), names)

    assert versions
    assert set(versions) == names
    assert set(versions.values()) == {"0.2.9"}
    assert 'version = "0.2.9"' in (fixture / "Cargo.toml").read_text()
    assert "tag: v0.2.9" in (fixture / "deploy/helm/tikeo/values.yaml").read_text()
