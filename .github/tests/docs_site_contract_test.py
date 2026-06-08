from pathlib import Path
import json
import unittest


ROOT = Path(__file__).resolve().parents[2]
WEBSITE = ROOT / "website"


class DocsSiteContractTest(unittest.TestCase):
    def test_website_scaffold_has_required_build_contract(self):
        package_json = WEBSITE / "package.json"
        self.assertTrue(package_json.exists(), "website/package.json must exist")
        package = json.loads(package_json.read_text())
        scripts = package.get("scripts", {})
        for script in ["docs:dev", "docs:build", "docs:serve", "docs:typecheck"]:
            self.assertIn(script, scripts)
        self.assertIn("@docusaurus/core", package.get("dependencies", {}))
        self.assertIn("@docusaurus/preset-classic", package.get("dependencies", {}))

    def test_website_config_exposes_bilingual_tikeo_navigation(self):
        config = (WEBSITE / "docusaurus.config.ts").read_text()
        self.assertIn("title: 'Tikeo'", config)
        self.assertIn("defaultLocale: 'en'", config)
        self.assertIn("'zh-CN'", config)
        for label in ["Docs", "SDKs", "Integrations", "GitHub"]:
            self.assertIn(f"label: '{label}'", config)

    def test_docs_information_architecture_contains_p0_pages(self):
        expected_pages = [
            "docs/index.md",
            "docs/getting-started/installation.md",
            "docs/getting-started/quickstart.md",
            "docs/getting-started/seed-demo-data.md",
            "docs/concepts/worker-tunnel.md",
            "docs/concepts/workflows.md",
            "docs/sdks/rust.md",
            "docs/sdks/go.md",
            "docs/sdks/java-spring-boot.md",
            "docs/deployment/docker-compose.md",
            "docs/deployment/kubernetes.md",
            "docs/reference/configuration.md",
            "docs/reference/troubleshooting.md",
        ]
        for relative_path in expected_pages:
            path = WEBSITE / relative_path
            self.assertTrue(path.exists(), f"missing docs page: {relative_path}")

        sidebars = (WEBSITE / "sidebars.ts").read_text()
        for section in [
            "Getting Started",
            "Core Concepts",
            "SDKs",
            "Deployment",
            "Integrations",
            "Reference",
        ]:
            self.assertIn(section, sidebars)

    def test_chinese_locale_and_llm_entrypoints_exist(self):
        zh_root = WEBSITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        self.assertTrue((zh_root / "index.md").exists())
        self.assertTrue((zh_root / "getting-started/quickstart.md").exists())
        self.assertTrue((WEBSITE / "static/llms.txt").exists())
        self.assertTrue((WEBSITE / "static/llms-full.txt").exists())


if __name__ == "__main__":
    unittest.main()
