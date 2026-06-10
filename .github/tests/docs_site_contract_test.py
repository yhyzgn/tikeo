from pathlib import Path
import re
import json
import unittest


ROOT = Path(__file__).resolve().parents[2]
DOCS_SITE = ROOT / "docs"
DOCS_ASSETS = ROOT / "assets" / "docs"
LEGACY_WEBSITE = ROOT / "website"
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
    "reference/management-openapi.md",
    "reference/worker-tunnel-protobuf.md",
    "reference/troubleshooting.md",
]

USER_GUIDE_DOCS = [
    "user-guide/dashboard.md",
    "user-guide/jobs.md",
    "user-guide/instances.md",
    "user-guide/workers.md",
    "user-guide/workflows.md",
    "user-guide/scripts.md",
    "user-guide/audit.md",
    "user-guide/settings.md",
]

SDK_MANAGEMENT_EXPECTATIONS = {
    "sdks/rust.md": [
        "ManagementClient::new",
        "ManagementCreateJobRequest::api",
        "ManagementTriggerJobRequest::api",
        "ManagementTriggerJobRequest::broadcast_api",
        "ManagementBroadcastSelectorRequest",
    ],
    "sdks/go.md": [
        "NewManagementClient",
        "APIJob",
        "APITrigger",
        "BroadcastAPITrigger",
        "BroadcastSelectorRequest",
    ],
    "sdks/java-spring-boot.md": [
        "HttpTikeoJobClient",
        "CreateJobRequest.api",
        "TriggerJobRequest.api",
        "TriggerJobRequest.broadcastApi",
        "BroadcastSelectorRequest",
    ],
    "sdks/python.md": [
        "ManagementClient",
        "api_job",
        "api_trigger",
        "broadcast_api_trigger",
        "BroadcastSelectorRequest",
    ],
    "sdks/nodejs.md": [
        "ManagementClient",
        "apiJob",
        "apiTrigger",
        "broadcastApiTrigger",
        "BroadcastSelectorRequest",
    ],
}

SDK_MANAGEMENT_COMMON_TOKENS = [
    "x-tikeo-api-key",
    "TIKEO_MANAGEMENT_API_KEY",
    "triggerType=api",
    "executionMode=single",
    "broadcastSelector",
]

REFERENCE_DOC_EXPECTATIONS = {
    "reference/management-openapi.md": [
        "crates/tikeo-server/src/http/openapi.rs",
        "crates/tikeo-server/src/http/router.rs",
        "/api-docs/openapi.json",
        "/api/v1/jobs",
        "/api/v1/jobs/{job}:trigger",
        "/api/v1/instances/{instance}",
        "/api/v1/instances/{instance}/logs",
        "CreateJobRequest",
        "TriggerJobRequest",
        "ApiResponse",
        "x-tikeo-api-key",
    ],
    "reference/worker-tunnel-protobuf.md": [
        "crates/tikeo-proto/proto/worker.proto",
        "package tikeo.worker.v1",
        "WorkerTunnelService",
        "OpenTunnel",
        "SubscribeTaskLogs",
        "RegisterWorker",
        "Heartbeat",
        "WorkerRegistered",
        "DispatchTask",
        "TaskLog",
        "TaskResult",
        "TaskCheckpoint",
        "assignment_token",
        "processor_name",
    ],
}

SDK_REFERENCE_LINK_TOKENS = [
    "../reference/management-openapi#post-api-v1-jobs",
    "../reference/management-openapi#post-api-v1-jobs-job-trigger",
    "../reference/management-openapi#get-api-v1-instances-instance",
    "../reference/management-openapi#get-api-v1-instances-instance-logs",
    "../reference/worker-tunnel-protobuf#dispatchtask",
]

DOCS_PUBLISHING_TOKENS = {
    "docusaurus.config.ts": [
        "TIKEO_DOCS_URL",
        "TIKEO_DOCS_BASE_URL",
        "headTags:",
        "og:title",
        "og:image",
        "twitter:card",
        "sitemap:",
    ],
    "static/robots.txt": [
        "User-agent: *",
        "Allow: /",
        "Sitemap: https://tikeo.dev/sitemap.xml",
    ],
    "static/search-index.json": [
        '"title": "Management OpenAPI reference"',
        '"/docs/reference/management-openapi"',
        '"/docs/reference/worker-tunnel-protobuf"',
        '"/docs/user-guide/jobs"',
        '"locale": "zh-CN"',
    ],
    "static/llms.txt": [
        "/docs/reference/management-openapi",
        "/docs/reference/worker-tunnel-protobuf",
        "/docs/user-guide/jobs",
        "/docs/user-guide/workers",
    ],
    "static/llms-full.txt": [
        "Generated from docs/docs and zh-CN docs",
        "Management OpenAPI reference",
        "Worker Tunnel protobuf reference",
        "Jobs user guide",
        "作业用户指南",
    ],
}

USER_GUIDE_EXPECTATIONS = {
    "user-guide/dashboard.md": ["Dashboard", "web/src/pages/Dashboard.tsx", "/api/v1/metrics/summary", "/api/v1/cluster"],
    "user-guide/jobs.md": ["Jobs", "web/src/pages/JobsPage.tsx", "/api/v1/jobs", "/api/v1/jobs/{job}:trigger"],
    "user-guide/instances.md": ["Instances", "web/src/pages/InstancesPage.tsx", "/api/v1/instances/{instance}", "/api/v1/instances/{instance}/logs"],
    "user-guide/workers.md": ["Workers", "web/src/pages/WorkersPage.tsx", "Worker Tunnel", "DispatchTask"],
    "user-guide/workflows.md": ["Workflows", "web/src/pages/WorkflowsPage.tsx", "/api/v1/workflows", "DAG"],
    "user-guide/scripts.md": ["Scripts", "web/src/pages/ScriptsPage.tsx", "/api/v1/scripts", "diff"],
    "user-guide/audit.md": ["Audit", "web/src/pages/AuditLogsPage.tsx", "/api/v1/audit-logs", "/api/v1/audit-logs:export"],
    "user-guide/settings.md": ["Settings", "web/src/routes.tsx", "API-Key", "RBAC"],
}


class DocsSiteContractTest(unittest.TestCase):

    def test_docs_site_module_replaces_legacy_website_module(self):
        self.assertTrue(DOCS_SITE.exists(), "docs/ must be the Docusaurus docs site module")
        self.assertFalse(LEGACY_WEBSITE.exists(), "legacy website/ module must be removed after migration to docs/")
        self.assertFalse((DOCS_SITE / "assets").exists(), "legacy docs/assets must move out before docs/ becomes the docs site")
        for asset in [
            "tikeo-logo-breathe.gif",
            "tikeo-console-tour.gif",
            "tikeo-architecture.en.svg",
            "tikeo-architecture.zh-CN.svg",
        ]:
            self.assertTrue((DOCS_ASSETS / asset).exists(), f"missing moved docs asset: assets/docs/{asset}")

    def test_docs_site_docker_image_contract_matches_web_style(self):
        dockerfile = DOCS_SITE / "Dockerfile"
        self.assertTrue(dockerfile.exists(), "docs/Dockerfile must build the docs site image")
        text = dockerfile.read_text()
        for token in [
            "FROM oven/bun:1.3.13 AS builder",
            "COPY package.json bun.lock ./",
            "bun install --frozen-lockfile",
            "NODE_ENV=production bun run docs:build",
            "FROM nginx:alpine AS runtime",
            "COPY --from=builder /app/build .",
            "COPY --from=builder /app/nginx/nginx.conf /etc/nginx/nginx.conf",
            "COPY --from=builder /app/nginx/default.conf /etc/nginx/conf.d/default.conf",
            'CMD ["nginx", "-g", "daemon off;"]',
        ]:
            self.assertIn(token, text)
        nginx_conf = DOCS_SITE / "nginx/nginx.conf"
        default_conf = DOCS_SITE / "nginx/default.conf"
        self.assertTrue(nginx_conf.exists(), "docs/nginx/nginx.conf must be copied into the image")
        self.assertTrue(default_conf.exists(), "docs/nginx/default.conf must be copied into the image")
        default_text = default_conf.read_text()
        self.assertIn("try_files $uri $uri/ /index.html", default_text)
        self.assertIn("location /healthz", default_text)
    def test_docs_site_scaffold_has_required_build_contract(self):
        package_json = DOCS_SITE / "package.json"
        self.assertTrue(package_json.exists(), "docs/package.json must exist")
        package = json.loads(package_json.read_text())
        scripts = package.get("scripts", {})
        for script in ["docs:dev", "docs:build", "docs:serve", "docs:typecheck"]:
            self.assertIn(script, scripts)
        self.assertIn("@docusaurus/core", package.get("dependencies", {}))
        self.assertIn("@docusaurus/preset-classic", package.get("dependencies", {}))

    def test_docs_lockfile_uses_public_registry_for_ci(self):
        lockfile = (DOCS_SITE / "bun.lock").read_text()
        self.assertNotIn("nexus3.recycloud.cn", lockfile)
        self.assertNotIn("repository/npm-public", lockfile)
        self.assertIn("https://registry.npmjs.org/", lockfile)

    def test_docs_site_config_exposes_bilingual_tikeo_navigation(self):
        config = (DOCS_SITE / "docusaurus.config.ts").read_text()
        self.assertIn("title: 'Tikeo'", config)
        self.assertIn("defaultLocale: 'en'", config)
        self.assertIn("'zh-CN'", config)
        for label in ["Docs", "SDKs", "Integrations", "GitHub"]:
            self.assertIn(f"label: '{label}'", config)

    def test_docs_config_supports_project_pages_base_url(self):
        config = (DOCS_SITE / "docusaurus.config.ts").read_text()
        homepage = (DOCS_SITE / "src/pages/index.tsx").read_text()
        self.assertIn("defaultLocale: 'en'", config)
        self.assertIn("TIKEO_DOCS_BASE_URL", config)
        self.assertIn("?? '/'", config)
        self.assertIn("useBaseUrl", homepage)
        self.assertIn("i18n.currentLocale === 'zh-CN'", homepage)

    def test_zh_navigation_sidebar_footer_are_localized(self):
        files = {
            "navbar": DOCS_SITE / "i18n/zh-CN/docusaurus-theme-classic/navbar.json",
            "footer": DOCS_SITE / "i18n/zh-CN/docusaurus-theme-classic/footer.json",
            "docs_options": DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current.json",
            "blog_options": DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-blog/options.json",
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
        blog_post = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-blog/2026-06-08-docs-site-scaffold.md"
        authors = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-blog/authors.yml"
        tags = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-blog/tags.yml"
        for path in [blog_post, authors, tags]:
            self.assertTrue(path.exists(), f"missing zh-CN blog localization: {path.name}")
        self.assertIn("文档站脚手架", blog_post.read_text())
        self.assertIn("Tikeo 维护者", authors.read_text())
        self.assertIn("发布说明与项目动态", tags.read_text())

    def test_docs_information_architecture_contains_p0_pages(self):
        for relative_path in P0_DOCS + USER_GUIDE_DOCS:
            path = DOCS_SITE / "docs" / relative_path
            self.assertTrue(path.exists(), f"missing docs page: docs/{relative_path}")

        sidebars = (DOCS_SITE / "sidebars.ts").read_text()
        for section in [
            "Getting Started",
            "Core Concepts",
            "User Guide",
            "SDKs",
            "Deployment",
            "Integrations",
            "Reference",
        ]:
            self.assertIn(section, sidebars)

    def test_chinese_locale_and_llm_entrypoints_exist(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for relative_path in P0_DOCS + USER_GUIDE_DOCS:
            self.assertTrue((zh_root / relative_path).exists(), f"missing zh-CN doc: {relative_path}")
        self.assertTrue((DOCS_SITE / "static/llms.txt").exists())
        self.assertTrue((DOCS_SITE / "static/llms-full.txt").exists())

    def test_p0_docs_have_enough_evaluation_depth(self):
        for relative_path in P0_DOCS:
            text = (DOCS_SITE / "docs" / relative_path).read_text()
            words = [word for word in text.replace("\n", " ").split(" ") if word.strip()]
            headings = [line for line in text.splitlines() if line.startswith("## ")]
            self.assertGreaterEqual(len(words), 260, f"doc too thin: {relative_path}")
            self.assertGreaterEqual(len(headings), 4, f"doc lacks sections: {relative_path}")

    def test_deployment_docs_include_copy_paste_runbooks(self):
        deployment_text = "\n".join(
            (DOCS_SITE / "docs" / relative_path).read_text()
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
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for relative_path in P0_DOCS + USER_GUIDE_DOCS:
            text = (zh_root / relative_path).read_text()
            cjk_chars = re.findall(r"[\u4e00-\u9fff]", text)
            headings = [line for line in text.splitlines() if line.startswith("## ")]
            self.assertGreaterEqual(len(cjk_chars), 300, f"zh-CN doc too thin: {relative_path}")
            self.assertGreaterEqual(len(headings), 4, f"zh-CN doc lacks sections: {relative_path}")

    def test_sdk_docs_include_source_backed_management_create_trigger_examples(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        roots = [DOCS_SITE / "docs", zh_root]
        for root in roots:
            for relative_path, specific_tokens in SDK_MANAGEMENT_EXPECTATIONS.items():
                text = (root / relative_path).read_text()
                for token in SDK_MANAGEMENT_COMMON_TOKENS + specific_tokens:
                    self.assertIn(token, text, f"{root.relative_to(DOCS_SITE)} / {relative_path} missing {token!r}")

    def test_reference_docs_are_source_backed_for_openapi_and_worker_proto(self):
        openapi_source = "\n".join(
            path.read_text()
            for path in [
                ROOT / "crates/tikeo-server/src/http/openapi.rs",
                ROOT / "crates/tikeo-server/src/http/router.rs",
                ROOT / "crates/tikeo-server/src/http/routes/jobs.rs",
            ]
        )
        proto_source = (ROOT / "crates/tikeo-proto/proto/worker.proto").read_text()
        for token in [
            "/api-docs/openapi.json",
            "/api/v1/jobs",
            "/api/v1/jobs/{job}:trigger",
            "/api/v1/instances/{instance}",
            "/api/v1/instances/{instance}/logs",
        ]:
            self.assertIn(token, openapi_source)
        for token in [
            "WorkerTunnelService",
            "OpenTunnel",
            "RegisterWorker",
            "DispatchTask",
            "TaskLog",
            "TaskResult",
            "TaskCheckpoint",
        ]:
            self.assertIn(token, proto_source)

        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for root in [DOCS_SITE / "docs", zh_root]:
            for relative_path, tokens in REFERENCE_DOC_EXPECTATIONS.items():
                path = root / relative_path
                self.assertTrue(path.exists(), f"missing reference doc: {path}")
                text = path.read_text()
                for token in tokens:
                    self.assertIn(token, text, f"{root.relative_to(DOCS_SITE)} / {relative_path} missing {token!r}")

        sidebars = (DOCS_SITE / "sidebars.ts").read_text()
        for item in ["reference/management-openapi", "reference/worker-tunnel-protobuf"]:
            self.assertIn(item, sidebars)

    def test_sdk_docs_link_helpers_to_exact_reference_anchors(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for root in [DOCS_SITE / "docs", zh_root]:
            for relative_path in SDK_MANAGEMENT_EXPECTATIONS:
                text = (root / relative_path).read_text()
                for token in SDK_REFERENCE_LINK_TOKENS:
                    self.assertIn(token, text, f"{root.relative_to(DOCS_SITE)} / {relative_path} missing {token!r}")

    def test_docs_publishing_search_and_seo_readiness(self):
        for relative_path, tokens in DOCS_PUBLISHING_TOKENS.items():
            text = (DOCS_SITE / relative_path).read_text()
            for token in tokens:
                self.assertIn(token, text, f"{relative_path} missing {token!r}")
        self.assertTrue((DOCS_SITE / "static/img/tikeo-og.png").exists(), "missing OpenGraph image")
        search_page = (DOCS_SITE / "src/pages/search.tsx").read_text()
        for token in ["search-index.json", "useBaseUrl", "fetch", "filteredEntries", "locale"]:
            self.assertIn(token, search_page, f"search page missing {token!r}")
        self.assertIn("label: 'Search'", (DOCS_SITE / "docusaurus.config.ts").read_text())
        sidebars = (DOCS_SITE / "sidebars.ts").read_text()
        for item in USER_GUIDE_DOCS:
            self.assertIn(item[:-3], sidebars, f"sidebar missing {item}")

    def test_user_guide_pages_are_source_backed(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for root in [DOCS_SITE / "docs", zh_root]:
            for relative_path, tokens in USER_GUIDE_EXPECTATIONS.items():
                text = (root / relative_path).read_text()
                headings = [line for line in text.splitlines() if line.startswith("## ")]
                self.assertGreaterEqual(len(headings), 4, f"{root.relative_to(DOCS_SITE)} / {relative_path} lacks sections")
                for token in tokens:
                    self.assertIn(token, text, f"{root.relative_to(DOCS_SITE)} / {relative_path} missing {token!r}")


if __name__ == "__main__":
    unittest.main()
