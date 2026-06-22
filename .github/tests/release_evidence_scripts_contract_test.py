#!/usr/bin/env python3
"""Contract checks for release-readiness evidence scripts and docs."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


class ReleaseEvidenceScriptsContractTest(unittest.TestCase):
    def test_scripts_exist_and_expose_expected_contracts(self) -> None:
        scripts = {
            "scripts/notification-provider-e2e-smoke.sh": [
                "notification-delivery-attempts:queue-status",
                "target redacted",
                "dead-letter",
                "provider-received.jsonl",
            ],
            "scripts/migration-cli-full-chain-smoke.sh": [
                "legacy-xxl-worker",
                "MIGRATE_BIN",
                "apply --bundle .tikeo-migration",
                "reviewed-import-payloads.json",
                "code-apply-evidence.json",
            ],
            "scripts/release-readiness-evidence.sh": [
                "TIKEO_CLOUD_HA_SERVER_URL",
                "passed_with_cloud_deferred",
                "cloud-ha-acceptance",
            ],
        }
        for script, needles in scripts.items():
            path = ROOT / script
            self.assertTrue(path.exists(), f"missing {script}")
            self.assertTrue(path.stat().st_mode & 0o111, f"{script} must be executable")
            text = path.read_text(encoding="utf-8")
            for needle in needles:
                self.assertIn(needle, text, f"{script} should mention {needle}")

    def test_docs_mention_release_evidence_commands(self) -> None:
        docs = [
            "docs/docs/development/product-readiness-acceptance.md",
            "docs/i18n/zh-CN/docusaurus-plugin-content-docs/current/development/product-readiness-acceptance.md",
            "docs/docs/development/release-acceptance-packet-v0.3.9.md",
            "docs/i18n/zh-CN/docusaurus-plugin-content-docs/current/development/release-acceptance-packet-v0.3.9.md",
        ]
        for doc in docs:
            text = read(doc)
            self.assertIn("notification-provider-e2e-smoke.sh", text, doc)
            self.assertIn("migration-cli-full-chain-smoke.sh", text, doc)
            self.assertIn("release-readiness-evidence.sh", text, doc)
            self.assertIn("TIKEO_CLOUD_HA_SERVER_URL", text, doc)


if __name__ == "__main__":
    unittest.main()
