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
        self.assertIn("defaultLocale: 'en'", config)
        self.assertIn("TIKEO_DOCS_BASE_URL", config)
        self.assertIn("?? '/'", config)
        self.assertIn("useBaseUrl", homepage)
        self.assertIn("i18n.currentLocale === 'zh-CN'", homepage)

    def test_zh_navigation_sidebar_footer_are_localized(self):
        files = {
            "navbar": WEBSITE / "i18n/zh-CN/docusaurus-theme-classic/navbar.json",
            "footer": WEBSITE / "i18n/zh-CN/docusaurus-theme-classic/footer.json",
            "docs_options": WEBSITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current.json",
            "blog_options": WEBSITE / "i18n/zh-CN/docusaurus-plugin-content-blog/options.json",
        }
        for name, path in files.items():
            self.assertTrue(path.exists(), f"missing zh-CN translation file: {name}")
        combined = "\n".join(path.read_text() for path in files.values())
        for localized in ["首页", "文档", "发布日志", "入门", "核心概念", "部署", "参考", "最近文章"]:
            self.assertIn(localized, combined)
        for untranslated in [
            '"message": "Home"',
            '"message": "Docs"',
            '"message": "Getting Started"',
            '"message": "Deployment"',
            '"message": "Recent posts"',
        ]:
            self.assertNotIn(untranslated, combined)

    def test_zh_blog_content_is_localized(self):
        blog_post = WEBSITE / "i18n/zh-CN/docusaurus-plugin-content-blog/2026-06-08-docs-site-scaffold.md"
        authors = WEBSITE / "i18n/zh-CN/docusaurus-plugin-content-blog/authors.yml"
        tags = WEBSITE / "i18n/zh-CN/docusaurus-plugin-content-blog/tags.yml"
        for path in [blog_post, authors, tags]:
            self.assertTrue(path.exists(), f"missing zh-CN blog localization: {path.name}")
        self.assertIn("文档站脚手架", blog_post.read_text())
        self.assertIn("Tikeo 维护者", authors.read_text())
        self.assertIn("发布说明与项目动态", tags.read_text())

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
