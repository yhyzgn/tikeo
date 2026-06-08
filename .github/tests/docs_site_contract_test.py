from pathlib import Path
import re
import json
import unittest


ROOT = Path(__file__).resolve().parents[2]
WEBSITE = ROOT / "website"
P0_DOCS = [
    "index.md",
    "getting-started/installation.md",
    "getting-started/quickstart.md",
    "getting-started/seed-demo-data.md",
    "concepts/worker-tunnel.md",
    "concepts/workflows.md",
    "sdks/rust.md",
    "sdks/go.md",
    "sdks/java-spring-boot.md",
    "sdks/python.md",
    "sdks/nodejs.md",
    "deployment/single-binary.md",
    "deployment/docker-compose.md",
    "deployment/kubernetes.md",
    "integrations/overview.md",
    "reference/configuration.md",
    "reference/troubleshooting.md",
]


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

    def test_docs_config_supports_project_pages_base_url(self):
        config = (WEBSITE / "docusaurus.config.ts").read_text()
        homepage = (WEBSITE / "src/pages/index.tsx").read_text()
        self.assertIn("TIKEO_DOCS_BASE_URL", config)
        self.assertIn("?? '/'", config)
        self.assertIn("useBaseUrl", homepage)

    def test_docs_information_architecture_contains_p0_pages(self):
        for relative_path in P0_DOCS:
            path = WEBSITE / "docs" / relative_path
            self.assertTrue(path.exists(), f"missing docs page: docs/{relative_path}")

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
        for relative_path in P0_DOCS:
            self.assertTrue((zh_root / relative_path).exists(), f"missing zh-CN doc: {relative_path}")
        self.assertTrue((WEBSITE / "static/llms.txt").exists())
        self.assertTrue((WEBSITE / "static/llms-full.txt").exists())

    def test_p0_docs_have_enough_evaluation_depth(self):
        for relative_path in P0_DOCS:
            text = (WEBSITE / "docs" / relative_path).read_text()
            words = [word for word in text.replace("\n", " ").split(" ") if word.strip()]
            headings = [line for line in text.splitlines() if line.startswith("## ")]
            self.assertGreaterEqual(len(words), 260, f"doc too thin: {relative_path}")
            self.assertGreaterEqual(len(headings), 4, f"doc lacks sections: {relative_path}")

    def test_deployment_docs_include_copy_paste_runbooks(self):
        deployment_text = "\n".join(
            (WEBSITE / "docs" / relative_path).read_text()
            for relative_path in [
                "deployment/single-binary.md",
                "deployment/docker-compose.md",
                "deployment/kubernetes.md",
                "reference/configuration.md",
            ]
        )
        for snippet in [
            "systemctl enable --now tikeo",
            "docker compose --env-file .env up -d --build",
            "Full `docker-compose.yml`",
            "Full `docker-compose.postgres.yml`",
            "Full `docker-compose.mysql.yml`",
            "helm upgrade --install tikeo",
            "kubectl -n tikeo create secret generic tikeo-database",
            "server.tls.workerTunnel.mtlsRequired",
            "TIKEO__STORAGE__DATABASE_URL",
            "server.worker_tunnel_addr",
        ]:
            self.assertIn(snippet, deployment_text)

    def test_zh_p0_docs_have_enough_localized_depth(self):
        zh_root = WEBSITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for relative_path in P0_DOCS:
            text = (zh_root / relative_path).read_text()
            cjk_chars = re.findall(r"[\u4e00-\u9fff]", text)
            headings = [line for line in text.splitlines() if line.startswith("## ")]
            self.assertGreaterEqual(len(cjk_chars), 300, f"zh-CN doc too thin: {relative_path}")
            self.assertGreaterEqual(len(headings), 4, f"zh-CN doc lacks sections: {relative_path}")


if __name__ == "__main__":
    unittest.main()
