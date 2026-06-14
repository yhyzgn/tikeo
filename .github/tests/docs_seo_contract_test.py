from __future__ import annotations

import html.parser
import json
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs"
BUILD = DOCS / "build"
CONFIG = DOCS / "docusaurus.config.ts"
GITHUB_SEO = ROOT / ".github" / "repository-seo.json"


class HeadParser(html.parser.HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.in_head = False
        self.in_json_ld = False
        self.meta: list[dict[str, str | None]] = []
        self.links: list[dict[str, str | None]] = []
        self.json_ld_chunks: list[str] = []
        self.json_ld_documents: list[dict[str, object]] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attributes = dict(attrs)
        if tag == "head":
            self.in_head = True
        if not self.in_head:
            return
        if tag == "meta":
            self.meta.append(attributes)
        elif tag == "link":
            self.links.append(attributes)
        elif tag == "script" and attributes.get("type") == "application/ld+json":
            self.in_json_ld = True
            self.json_ld_chunks = []

    def handle_endtag(self, tag: str) -> None:
        if tag == "script" and self.in_json_ld:
            self.in_json_ld = False
            document = json.loads("".join(self.json_ld_chunks))
            self.json_ld_documents.append(document)
        elif tag == "head":
            self.in_head = False

    def handle_data(self, data: str) -> None:
        if self.in_json_ld:
            self.json_ld_chunks.append(data)

    def meta_value(self, *, name: str | None = None, property_: str | None = None) -> str | None:
        for item in self.meta:
            if name is not None and item.get("name") == name:
                return item.get("content")
            if property_ is not None and item.get("property") == property_:
                return item.get("content")
        return None

    def links_for(self, rel: str) -> list[dict[str, str | None]]:
        return [item for item in self.links if item.get("rel") == rel]


def parse_head(relative: str) -> HeadParser:
    path = BUILD / relative
    if not path.exists():
        raise AssertionError(f"missing built page: {path}")
    parser = HeadParser()
    parser.feed(path.read_text(encoding="utf-8"))
    return parser


class DocsSeoContractTest(unittest.TestCase):
    def test_config_declares_search_friendly_metadata(self) -> None:
        config = CONFIG.read_text()
        for token in [
            "titleDelimiter: '·'",
            "rel: 'manifest'",
            "rel: 'search'",
            "opensearch.xml",
            "name: 'keywords'",
            "distributed task scheduler",
            "workflow orchestration",
            "Worker Tunnel",
            "XXL-Job alternative",
            "PowerJob alternative",
            "name: 'robots'",
            "max-image-preview:large",
            "property: 'og:site_name'",
            "property: 'og:type'",
            "property: 'og:image:alt'",
            "name: 'twitter:title'",
            "name: 'twitter:description'",
            "rel: 'sitemap'",
            "SoftwareApplication",
            "SearchAction",
            "codeRepository",
        ]:
            self.assertIn(token, config)

    def test_built_homepage_has_complete_seo_head(self) -> None:
        head = parse_head("index.html")
        description = head.meta_value(name="description") or ""
        keywords = head.meta_value(name="keywords") or ""
        self.assertIn("distributed task scheduling", description)
        self.assertIn("Worker Tunnel", description)
        for keyword in ["distributed task scheduler", "workflow orchestration", "Worker Tunnel", "Kubernetes operator", "XXL-Job alternative"]:
            self.assertIn(keyword, keywords)

        self.assertEqual(head.meta_value(name="robots"), "index,follow,max-image-preview:large,max-snippet:-1,max-video-preview:-1")
        self.assertEqual(head.meta_value(property_="og:type"), "website")
        self.assertEqual(head.meta_value(property_="og:site_name"), "Tikeo Documentation")
        self.assertEqual(head.meta_value(property_="og:image:alt"), "Tikeo task orchestration documentation preview")
        self.assertEqual(head.meta_value(name="twitter:card"), "summary_large_image")
        self.assertIn("Tikeo", head.meta_value(name="twitter:title") or "")
        self.assertIn("Worker Tunnel", head.meta_value(name="twitter:description") or "")

        canonicals = head.links_for("canonical")
        self.assertEqual(canonicals[0].get("href"), "https://tikeo.dev/")
        alternates = {(link.get("hreflang"), link.get("href")) for link in head.links_for("alternate")}
        self.assertIn(("en", "https://tikeo.dev/"), alternates)
        self.assertIn(("zh-CN", "https://tikeo.dev/zh-CN/"), alternates)
        self.assertIn(("x-default", "https://tikeo.dev/"), alternates)
        self.assertTrue(any(link.get("rel") == "manifest" and link.get("href") == "https://tikeo.dev/site.webmanifest" for link in head.links))
        self.assertTrue(any(link.get("rel") == "search" and link.get("type") == "application/opensearchdescription+xml" for link in head.links))
        self.assertTrue(any(link.get("rel") == "sitemap" and link.get("href") == "https://tikeo.dev/sitemap.xml" for link in head.links))

    def test_json_ld_contains_software_website_and_org_entities(self) -> None:
        head = parse_head("index.html")
        by_type = {doc.get("@type"): doc for doc in head.json_ld_documents}
        self.assertIn("Organization", by_type)
        self.assertIn("SoftwareApplication", by_type)
        self.assertIn("WebSite", by_type)
        software = by_type["SoftwareApplication"]
        self.assertEqual(software.get("name"), "Tikeo")
        self.assertEqual(software.get("codeRepository"), "https://github.com/yhyzgn/tikeo")
        self.assertIn("Worker Tunnel", software.get("description", ""))
        self.assertIn("Rust", software.get("programmingLanguage", []))
        website = by_type["WebSite"]
        action = website.get("potentialAction", {})
        self.assertIsInstance(action, dict)
        self.assertEqual(action.get("@type"), "SearchAction")
        self.assertIn("search?q={search_term_string}", action.get("target", ""))

    def test_zh_homepage_has_canonical_hreflang_and_localized_description(self) -> None:
        head = parse_head("zh-CN/index.html")
        self.assertEqual(head.meta_value(name="docusaurus_locale"), "zh-CN")
        self.assertIn("Tikeo", head.meta_value(name="description") or "")
        canonicals = head.links_for("canonical")
        self.assertEqual(canonicals[0].get("href"), "https://tikeo.dev/zh-CN/")
        alternates = {(link.get("hreflang"), link.get("href")) for link in head.links_for("alternate")}
        self.assertIn(("en", "https://tikeo.dev/"), alternates)
        self.assertIn(("zh-CN", "https://tikeo.dev/zh-CN/"), alternates)

    def test_robots_sitemap_manifest_and_llms_are_indexer_ready(self) -> None:
        robots = (BUILD / "robots.txt").read_text()
        self.assertIn("User-agent: *", robots)
        self.assertIn("Allow: /", robots)
        self.assertIn("Sitemap: https://tikeo.dev/sitemap.xml", robots)
        self.assertIn("Host: https://tikeo.dev", robots)
        self.assertTrue((BUILD / "site.webmanifest").exists())
        manifest = json.loads((BUILD / "site.webmanifest").read_text())
        self.assertEqual(manifest["name"], "Tikeo Documentation")
        self.assertEqual(manifest["theme_color"], "#3157d5")
        opensearch = (BUILD / "opensearch.xml").read_text()
        self.assertIn("Tikeo Docs", opensearch)
        self.assertIn("https://tikeo.dev/search?q={searchTerms}", opensearch)

        en_sitemap = ET.parse(BUILD / "sitemap.xml")
        en_urls = {element.text for element in en_sitemap.findall(".//{http://www.sitemaps.org/schemas/sitemap/0.9}loc")}
        for expected in [
            "https://tikeo.dev/",
            "https://tikeo.dev/docs/getting-started/quickstart",
            "https://tikeo.dev/docs/reference/configuration",
            "https://tikeo.dev/docs/user-guide/notifications",
        ]:
            self.assertIn(expected, en_urls)

        zh_sitemap = ET.parse(BUILD / "zh-CN/sitemap.xml")
        zh_urls = {element.text for element in zh_sitemap.findall(".//{http://www.sitemaps.org/schemas/sitemap/0.9}loc")}
        self.assertIn("https://tikeo.dev/zh-CN/", zh_urls)
        self.assertIn("https://tikeo.dev/zh-CN/docs/getting-started/quickstart", zh_urls)

        llms = (BUILD / "llms.txt").read_text()
        self.assertIn("distributed task scheduling", llms)
        self.assertIn("/docs/user-guide/notifications", llms)
        self.assertIn("/zh-CN/docs/user-guide/notifications", llms)

    def test_github_repository_seo_contract_is_versioned(self) -> None:
        metadata = json.loads(GITHUB_SEO.read_text())
        self.assertEqual(metadata["repository"], "yhyzgn/tikeo")
        self.assertEqual(metadata["homepageUrl"], "https://tikeo.dev")
        self.assertLessEqual(len(metadata["description"]), 350)
        for topic in [
            "task-scheduler",
            "distributed-scheduler",
            "workflow-orchestration",
            "workflow-engine",
            "job-scheduler",
            "worker-tunnel",
            "rust",
            "kubernetes",
            "opentelemetry",
            "xxl-job",
            "powerjob",
        ]:
            self.assertIn(topic, metadata["topics"])
        self.assertGreaterEqual(len(metadata["topics"]), 20)
        self.assertLessEqual(len(metadata["topics"]), 20)
        readme = (ROOT / "README.md").read_text()
        readme_zh = (ROOT / "README.zh-CN.md").read_text()
        self.assertIn("https://tikeo.dev", readme)
        self.assertIn("https://tikeo.dev/zh-CN/", readme_zh)


if __name__ == "__main__":
    unittest.main()
