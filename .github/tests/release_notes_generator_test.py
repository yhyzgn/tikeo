#!/usr/bin/env python3
"""Contract tests for product-style release note generation."""

import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "generate-release-notes.py"

spec = importlib.util.spec_from_file_location("generate_release_notes", SCRIPT)
release_notes = importlib.util.module_from_spec(spec)
assert spec.loader is not None
import sys
sys.modules[spec.name] = release_notes
spec.loader.exec_module(release_notes)


class ReleaseNotesGeneratorTest(unittest.TestCase):
    def test_product_notes_are_structured_and_not_bilingual_dump(self):
        commits = [
            release_notes.Commit(
                "abc1234",
                "2026-06-18",
                "Generate product-style release notes",
                ["scripts/generate-release-notes.py", ".github/workflows/release-github-assets.yml"],
            ),
            release_notes.Commit(
                "def5678",
                "2026-06-18",
                "Trim migration CLI source size",
                ["crates/tikeo-migrate/src/lib.rs"],
            ),
            release_notes.Commit(
                "fedcba9",
                "2026-06-18",
                "Fix release asset workflow heredocs",
                [".github/workflows/release-github-assets.yml"],
            ),
        ]
        assets = [
            "go-sdk-0.3.6.tar.gz",
            "java-sdk-0.3.6.tar.gz",
            "nodejs-sdk-0.3.6.tar.gz",
            "python-sdk-0.3.6.tar.gz",
            "rust-sdk-0.3.6.tar.gz",
            "tikeo-deploy-sources-0.3.6.tar.gz",
            "tikeo-migrate-0.3.6-x86_64-pc-windows-msvc.zip",
            "tikeo-server-0.3.6-x86_64-unknown-linux-gnu.tar.gz",
        ]

        body = release_notes.render_notes("v0.3.6", "v0.3.5", commits, assets)

        for heading in [
            "## Highlights",
            "## Downloads",
            "## Added",
            "## Changed",
            "## Fixed",
            "## Upgrade notes",
            "## Verification",
            "## Commit audit",
        ]:
            self.assertIn(heading, body)
        self.assertNotIn("## English", body)
        self.assertNotIn("## 中文", body)
        self.assertIn("Release experience", body)
        self.assertIn("Migration toolkit", body)
        self.assertIn("Deployment source bundle", body)
        self.assertIn("SDK source package | Go", body)
        self.assertIn("SDK source package | Java", body)
        self.assertIn("SDK source package | Node.js", body)
        self.assertIn("SDK source package | Python", body)
        self.assertIn("SDK source package | Rust", body)
        self.assertIn("Upgrades the GitHub Release page into a product-oriented summary", body)
        self.assertIn("Keeps the migration CLI implementation within repository quality gates", body)

    def test_agent_context_commits_stay_out_of_user_facing_sections(self):
        commits = [
            release_notes.Commit("aaa1111", "2026-06-18", "Update project agent context", ["AGENTS.md"]),
            release_notes.Commit(
                "bbb2222",
                "2026-06-18",
                "Stabilize shard owner SLO test data",
                ["crates/tikeo-storage/src/repository/tests/part_03.rs"],
            ),
        ]

        body = release_notes.render_notes("v0.3.7", "v0.3.6", commits, [])
        highlights = body.split("## Downloads", 1)[0]
        user_facing_changes = body.split("## Upgrade notes", 1)[0]
        audit = body.split("## Commit audit", 1)[1]

        self.assertNotIn("Update project agent context", highlights)
        self.assertNotIn("Update project agent context", user_facing_changes)
        self.assertIn("Server & scheduling", highlights)
        self.assertIn("Stabilize shard owner SLO test data", user_facing_changes)
        self.assertIn("Update project agent context", audit)

    def test_asset_table_is_ordered_by_operator_use(self):
        assets = [
            "rust-sdk-0.3.6.tar.gz",
            "tikeo-deploy-sources-0.3.6.tar.gz",
            "tikeo-server-0.3.6-x86_64-unknown-linux-gnu.tar.gz",
            "tikeo-migrate-0.3.6-x86_64-unknown-linux-gnu.tar.gz",
        ]
        ordered = release_notes.list_assets_from_names_for_test(assets) if hasattr(release_notes, "list_assets_from_names_for_test") else sorted(assets, key=release_notes.asset_order)
        self.assertEqual(
            ordered,
            [
                "tikeo-server-0.3.6-x86_64-unknown-linux-gnu.tar.gz",
                "tikeo-migrate-0.3.6-x86_64-unknown-linux-gnu.tar.gz",
                "rust-sdk-0.3.6.tar.gz",
                "tikeo-deploy-sources-0.3.6.tar.gz",
            ],
        )


if __name__ == "__main__":
    unittest.main()
