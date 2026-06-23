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
    "deployment/server-ha.md",
    "deployment/kubernetes-controller-runbook.md",
    "deployment/management-trigger-smoke-runbook.md",
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
        "Sitemap: https://docs.tikeo.net/sitemap.xml",
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

FORBIDDEN_PUBLIC_DOC_TERMS = [
    "source-backed",
    "source-derived",
    "docs slice",
    "hallucinated",
    "memory/prompt",
    "prompt handoff",
    "Contributor",
    "源码事实",
]

HUMAN_MANUAL_EN_DOCS = [
    "index.md",
    "getting-started/installation.md",
    "getting-started/quickstart.md",
    "getting-started/seed-demo-data.md",
    "deployment/docker-compose.md",
    "deployment/kubernetes.md",
    "deployment/server-ha.md",
    "deployment/sse-realtime.md",
    "reference/configuration.md",
    "reference/troubleshooting.md",
    "integrations/overview.md",
    "user-guide/dashboard.md",
    "user-guide/jobs.md",
    "user-guide/instances.md",
    "user-guide/workers.md",
    "user-guide/workflows.md",
    "user-guide/scripts.md",
    "user-guide/audit.md",
    "user-guide/settings.md",
    "user-guide/notifications.md",
    "user-guide/alerts.md",
    "sdks/rust.md",
    "sdks/go.md",
    "sdks/java-spring-boot.md",
    "sdks/python.md",
    "sdks/nodejs.md",
]

HUMAN_MANUAL_ZH_DOCS = HUMAN_MANUAL_EN_DOCS

HUMAN_MANUAL_EN_TOKENS = ["Prerequisites", "Verify", "Troubleshooting", "Production checklist"]
HUMAN_MANUAL_ZH_TOKENS = ["前置条件", "验收", "故障排查", "生产检查清单"]

BILINGUAL_MIN_SECTION_DOCS = [
    "index.md",
    "getting-started/installation.md",
    "getting-started/quickstart.md",
    "getting-started/seed-demo-data.md",
    "deployment/docker-compose.md",
    "deployment/kubernetes.md",
    "deployment/server-ha.md",
    "deployment/sse-realtime.md",
    "reference/configuration.md",
    "reference/troubleshooting.md",
    "integrations/overview.md",
    "user-guide/jobs.md",
    "user-guide/workers.md",
    "user-guide/instances.md",
    "user-guide/settings.md",
    "user-guide/notifications.md",
    "sdks/rust.md",
    "sdks/go.md",
    "sdks/java-spring-boot.md",
    "sdks/python.md",
    "sdks/nodejs.md",
]

NOTIFICATION_CENTER_DOC_TOKENS = [
    "crates/tikeo-server/src/http/routes/notifications.rs",
    "crates/tikeo-server/src/http/routes/notification_templates.rs",
    "notification_templates",
    "/api/v1/notification-templates",
    "/api/v1/notification-templates/{id}/render",
    "templateRef",
    "blockKit",
    "actionCard",
    "feedCard",
    "interactive",
    "share_chat",
    "markdown_v2",
    "template_card",
    "PagerDuty",
    "supportsTestSend=true",
]

NOTIFICATION_CENTER_SOURCE_TOKENS = [
    "create_notification_templates",
    "render_notification_template",
    "validate_provider_message_template",
    "builtin_channel_template",
    "blockKit",
    "actionCard",
    "feedCard",
    "interactive",
    "share_chat",
    "markdown_v2",
    "template_card",
    "supports_test_send: false",
]


class DocsSiteContractTest(unittest.TestCase):

    def test_readme_release_badges_use_github_release_source_of_truth(self):
        release_badge_url = "img.shields.io/github/v/release/yhyzgn/tikeo"
        stale_release_badge_asset = "docs/static/release-badge.json"
        stale_dynamic_badge_fragment = "raw.githubusercontent.com%2Fyhyzgn%2Ftikeo%2Fmain%2Fdocs%2Fstatic%2Frelease-badge.json"

        for readme in [ROOT / "README.md", ROOT / "README.zh-CN.md"]:
            text = readme.read_text(encoding="utf-8")
            self.assertIn(release_badge_url, text)
            self.assertNotIn("img.shields.io/badge/release-v", text)
            self.assertNotIn(stale_release_badge_asset, text)
            self.assertNotIn(stale_dynamic_badge_fragment, text)

        self.assertFalse(
            (ROOT / stale_release_badge_asset).exists(),
            "release badge must come from GitHub releases, not a stale generated JSON asset",
        )


    def test_docs_site_module_replaces_legacy_website_module(self):
        self.assertTrue(DOCS_SITE.exists(), "docs/ must be the Docusaurus docs site module")
        self.assertFalse(LEGACY_WEBSITE.exists(), "legacy website/ module must be removed after migration to docs/")
        self.assertFalse((DOCS_SITE / "assets").exists(), "legacy docs/assets must move out before docs/ becomes the docs site")
        for asset in [
            "tikeo-logo.svg",
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
        self.assertIn("absolute_redirect off", default_text)
        self.assertIn("port_in_redirect off", default_text)
        self.assertIn("try_files $uri $uri/ /index.html", default_text)
        self.assertIn("location /healthz", default_text)

    def test_docs_site_scaffold_has_required_build_contract(self):
        package_json = DOCS_SITE / "package.json"
        self.assertTrue(package_json.exists(), "docs/package.json must exist")
        package = json.loads(package_json.read_text())
        scripts = package.get("scripts", {})
        for script in ["start", "docs:dev", "docs:dev:en", "docs:dev:zh", "docs:build", "docs:serve", "docs:typecheck"]:
            self.assertIn(script, scripts)
        self.assertEqual(scripts["start"], "bun run docs:dev")
        self.assertIn("bun run docs:build", scripts["docs:dev"])
        self.assertIn("docusaurus serve", scripts["docs:dev"])
        self.assertNotEqual(scripts["docs:dev"], "docusaurus start --host 0.0.0.0")
        self.assertIn("--locale en", scripts["docs:dev:en"])
        self.assertIn("--locale zh-CN", scripts["docs:dev:zh"])
        self.assertIn("@docusaurus/core", package.get("dependencies", {}))
        self.assertIn("@docusaurus/preset-classic", package.get("dependencies", {}))

    def test_docs_lockfile_uses_public_registry_for_ci(self):
        lockfile = (DOCS_SITE / "bun.lock").read_text()
        self.assertNotIn("nexus3." + "recy" + "cloud" + ".cn", lockfile)
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
        self.assertNotIn("TikeoLogoMark", homepage)
        self.assertIn("TikeoLogo", homepage)
        styles = (DOCS_SITE / "src/pages/index.module.css").read_text()
        self.assertIn("i18n.currentLocale === 'zh-CN'", homepage)
        self.assertIn("styles.titleZh", homepage)
        self.assertIn("line-height: 1.16", styles)
        self.assertIn("letter-spacing: -0.025em", styles)
        self.assertNotIn("tikeo-logo-breathe.gif", config + homepage)
        self.assertNotIn("照着部署、接入 Worker、配置系统", homepage)

    def test_docs_logo_uses_copied_web_console_svg_asset(self):
        config = (DOCS_SITE / "docusaurus.config.ts").read_text()
        homepage = (DOCS_SITE / "src/pages/index.tsx").read_text()
        readme = (ROOT / "README.md").read_text() + (ROOT / "README.zh-CN.md").read_text()
        web_logo = (ROOT / "web/src/assets/tikeo-logo.svg").read_text()
        docs_logo = (DOCS_SITE / "static/img/tikeo-logo.svg").read_text()
        readme_logo = (ROOT / "assets/docs/tikeo-logo.svg").read_text()
        logo_component = (DOCS_SITE / "src/components/TikeoLogo/index.tsx").read_text()
        logo_styles = (DOCS_SITE / "src/components/TikeoLogo/styles.css").read_text()
        self.assertIn("img/tikeo-logo.svg", config)
        self.assertIn("TikeoLogo", homepage)
        self.assertNotIn("TikeoLogoMark", homepage)
        for token in [
            'viewBox="4 4 56 56"',
            'M32 5.5L54.5 18.5V45.5L32 58.5L9.5 45.5V18.5L32 5.5Z',
            'M32 13L47 21.5V42.5L32 51L17 42.5V21.5L32 13Z',
            'M19 25.5H44',
            'M32 25.5V45',
            'M32 35.5H46',
            'M41 31L47 35.5L41 40',
        ]:
            self.assertIn(token, logo_component)
        for token in [
            "--app-primary-color: var(--tikeo-docs-logo-primary, #2563eb)",
            "--app-info-color: var(--tikeo-docs-logo-info, #0ea5e9)",
            "--tikeo-logo-accent: var(--tikeo-docs-logo-accent, #7c3aed)",
            "tikeo-logo-flow 2s cubic-bezier(0.45, 0, 0.2, 1) infinite",
            "tikeo-logo-node-pulse 2s ease-in-out infinite",
            "tikeo-logo-arrow 2s cubic-bezier(0.45, 0, 0.2, 1) infinite",
            "tikeo-logo-breathe 5.2s ease-in-out infinite",
            "prefers-reduced-motion: reduce",
        ]:
            self.assertIn(token, logo_styles)
        self.assertEqual(docs_logo, web_logo)
        self.assertEqual(readme_logo, web_logo)
        self.assertIn("assets/docs/tikeo-logo.svg", readme)
        self.assertNotIn("tikeo-logo-breathe.gif", config + homepage + readme)
        self.assertNotIn("orbital", config + homepage + readme)

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


    def test_server_ha_docs_explain_architecture_and_tradeoffs(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for root, title in [(DOCS_SITE / "docs", "Server HA and Raft FSOD Cluster"), (zh_root, "Server 高可用与 Raft FSOD 集群")]:
            text = (root / "deployment/server-ha.md").read_text()
            for token in [
                "```mermaid",
                "StatefulSet",
                "headless",
                "Worker Tunnel",
                "gateway_node_id",
                "internal relay",
                "cluster.transport_token",
                "canSchedule=true",
                "Redis",
                "Dragonfly",
                "shard",
                "fencing",
            ]:
                self.assertIn(token, text, f"{title} missing {token!r}")
            self.assertGreaterEqual(text.count("```mermaid"), 3, f"{title} should include architecture, decision, and failover diagrams")

        readme = (ROOT / "README.md").read_text()
        readme_zh = (ROOT / "README.zh-CN.md").read_text()
        search_index = (DOCS_SITE / "static/search-index.json").read_text()
        llms = (DOCS_SITE / "static/llms.txt").read_text()
        self.assertIn("https://docs.tikeo.net/docs/deployment/server-ha", readme)
        self.assertIn("https://docs.tikeo.net/zh-CN/docs/deployment/server-ha", readme_zh)
        self.assertNotIn("docs/docs/deployment/server-ha.md", readme)
        self.assertNotIn("docs/i18n/zh-CN/docusaurus-plugin-content-docs/current/deployment/server-ha.md", readme_zh)
        self.assertIn("Raft FSOD Cluster", readme)
        self.assertIn("More Server pods improve failover, Worker Tunnel distribution, and dispatch throughput for owned shards", readme)
        self.assertIn("/docs/deployment/server-ha", search_index)
        self.assertIn("/zh-CN/docs/deployment/server-ha", search_index)
        self.assertIn("/docs/deployment/server-ha", llms)
        self.assertIn("/zh-CN/docs/deployment/server-ha", llms)
        sidebars = (DOCS_SITE / "sidebars.ts").read_text()
        index_en = (DOCS_SITE / "docs/index.md").read_text()
        index_zh = (zh_root / "index.md").read_text()
        self.assertIn("type: 'generated-index'", sidebars)
        self.assertIn("Server HA and Raft FSOD Cluster", index_en)
        self.assertIn("Server 高可用与 Raft FSOD 集群", index_zh)

    def test_product_readiness_acceptance_links_core_surfaces(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        expectations = [
            (DOCS_SITE / "docs" / "development/product-readiness-acceptance.md", [
                "Notification Center",
                "tikeo-migrate",
                "Raft FSOD Server HA",
                "provider test-send",
                "reviewed-import-payloads",
                "scripts/kind-raft-ha-e2e.sh",
                "scripts/verify-raft-ha-rollout.sh",
                "scripts/cloud-raft-ha-acceptance.sh",
                "Cross-language Worker soak",
                "TIKEO_CROSS_SOAK_SECONDS=120",
                ".github/workflows/release-candidate-worker-soak.yml",
                "cross-language-worker-soak",
                "*-soak-summary.json",
                "*-soak-metrics.jsonl",
                "worker_dispatch_outbox",
                "Product readiness acceptance checklist",
            ]),
            (zh_root / "development/product-readiness-acceptance.md", [
                "通知中心",
                "tikeo-migrate",
                "Raft FSOD Server HA",
                "provider 测试",
                "reviewed-import-payloads",
                "scripts/kind-raft-ha-e2e.sh",
                "scripts/verify-raft-ha-rollout.sh",
                "scripts/cloud-raft-ha-acceptance.sh",
                "跨语言 Worker soak",
                "TIKEO_CROSS_SOAK_SECONDS=120",
                ".github/workflows/release-candidate-worker-soak.yml",
                "cross-language-worker-soak",
                "*-soak-summary.json",
                "*-soak-metrics.jsonl",
                "worker_dispatch_outbox",
                "产品就绪验收清单",
            ]),
        ]
        for path, tokens in expectations:
            self.assertTrue(path.exists(), f"missing product readiness checklist: {path}")
            text = path.read_text()
            for token in tokens:
                self.assertIn(token, text, f"{path.relative_to(DOCS_SITE)} missing {token!r}")

        readme = (ROOT / "README.md").read_text()
        readme_zh = (ROOT / "README.zh-CN.md").read_text()
        sidebars = (DOCS_SITE / "sidebars.ts").read_text()
        search_index = (DOCS_SITE / "static/search-index.json").read_text()
        llms = (DOCS_SITE / "static/llms.txt").read_text() + (DOCS_SITE / "static/llms-full.txt").read_text()
        for token in [
            "/docs/development/product-readiness-acceptance",
            "/zh-CN/docs/development/product-readiness-acceptance",
            "/docs/development/release-acceptance-packet-v0.3.10",
            "/zh-CN/docs/development/release-acceptance-packet-v0.3.10",
            "/docs/development/release-acceptance-packet-v0.3.9",
            "/zh-CN/docs/development/release-acceptance-packet-v0.3.9",
        ]:
            self.assertIn(token, readme + readme_zh + search_index + llms)
        self.assertIn("development/product-readiness-acceptance", sidebars)
        self.assertIn("development/release-acceptance-packet-v0.3.10", sidebars)
        self.assertIn("development/release-acceptance-packet-v0.3.9", sidebars)

    def test_deployment_docs_include_copy_paste_runbooks(self):
        deployment_text = "\n".join(
            (DOCS_SITE / "docs" / relative_path).read_text()
            for relative_path in [
                "deployment/single-binary.md",
                "deployment/docker-compose.md",
                "deployment/kubernetes.md",
                "deployment/server-ha.md",
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

    def test_management_trigger_smoke_runbook_is_source_backed(self):
        script = (ROOT / "scripts/management-trigger-e2e-smoke.sh").read_text()
        for source_token in [
            "TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER",
            "TIKEO_MANAGEMENT_TRIGGER_REPORT_DIR",
            "TIKEO_MANAGEMENT_TRIGGER_RUN_ID",
            "management-sdk-create-trigger",
            "management-instance-result",
            "tikeo_smoke_finalize_report",
            "nodejs demo echo processed",
        ]:
            self.assertIn(source_token, script)

        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for root in [DOCS_SITE / "docs", zh_root]:
            text = (root / "deployment/management-trigger-smoke-runbook.md").read_text()
            for token in [
                "scripts/management-trigger-e2e-smoke.sh",
                "TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh",
                "TIKEO_MANAGEMENT_TRIGGER_REPORT_DIR",
                "TIKEO_MANAGEMENT_TRIGGER_RUN_ID",
                "management-sdk-create-trigger",
                "management-instance-result",
                "management trigger e2e report:",
                ".dev/reports/management-trigger-e2e-",
                "x-tikeo-api-key",
                "TIKEO_WORKER_CONNECT=1",
                "nodejs demo echo processed",
            ]:
                self.assertIn(token, text, f"{root.relative_to(DOCS_SITE)} management runbook missing {token!r}")

    def test_kubernetes_controller_runbook_is_controller_specific_and_source_backed(self):
        source_bundle = "\n".join(
            path.read_text()
            for path in [
                ROOT / "deploy/helm/tikeo/values.yaml",
                ROOT / "deploy/helm/tikeo/examples/values-ingress-tls.yaml",
                ROOT / "deploy/helm/tikeo/examples/values-gateway-api-worker-tunnel.yaml",
                ROOT / "deploy/helm/tikeo/templates/server.yaml",
                ROOT / "deploy/helm/tikeo/templates/gateway-api.yaml",
                ROOT / "deploy/helm/tikeo/templates/networkpolicy.yaml",
            ]
        )
        for source_token in [
            "workerTunnelService",
            "workerTunnel:",
            "mtlsRequired: true",
            "nginx.ingress.kubernetes.io/backend-protocol",
            "gatewayApi:",
            "kind: GRPCRoute",
            "tikeo.yhyzgn.com/worker-networking",
            "workers-connect-outbound-only",
        ]:
            self.assertIn(source_token, source_bundle)

        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for root in [DOCS_SITE / "docs", zh_root]:
            text = (root / "deployment/kubernetes-controller-runbook.md").read_text()
            headings = [line for line in text.splitlines() if line.startswith("## ")]
            self.assertGreaterEqual(len(headings), 6, f"{root.relative_to(DOCS_SITE)} controller runbook lacks sections")
            for token in [
                "Nginx Ingress",
                "Envoy Gateway",
                "Traefik",
                "Gateway API",
                "server.ingress.className",
                "server.workerTunnelService.annotations",
                "server.tls.workerTunnel.mtlsRequired",
                "gatewayApi.enabled",
                "GRPCRoute",
                "grpc-worker-tunnel",
                "workers-connect-outbound-only",
                "values-ingress-tls.yaml",
                "values-gateway-api-worker-tunnel.yaml",
                "curl -fsS",
                "kubectl -n tikeo",
            ]:
                self.assertIn(token, text, f"{root.relative_to(DOCS_SITE)} controller runbook missing {token!r}")

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

    def test_public_docs_are_written_for_humans_not_ai_handoffs(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for root in [DOCS_SITE / "docs", zh_root]:
            for path in root.rglob("*.md"):
                text = path.read_text()
                for term in FORBIDDEN_PUBLIC_DOC_TERMS:
                    self.assertNotIn(term, text, f"public doc {path.relative_to(root)} contains internal handoff term {term!r}")
                self.assertNotIn("curl -fsS http://0.0.0.0", text, f"public doc must curl 127.0.0.1 or a real host: {path}")
                self.assertNotIn("http://0.0.0.0", text, f"public doc must not use 0.0.0.0 as a client URL: {path}")

    def test_priority_manual_docs_have_human_operational_sections(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for relative_path in HUMAN_MANUAL_EN_DOCS:
            text = (DOCS_SITE / "docs" / relative_path).read_text()
            for token in HUMAN_MANUAL_EN_TOKENS:
                self.assertIn(token, text, f"{relative_path} must read like an operator manual and include {token!r}")
        for relative_path in HUMAN_MANUAL_ZH_DOCS:
            text = (zh_root / relative_path).read_text()
            for token in HUMAN_MANUAL_ZH_TOKENS:
                self.assertIn(token, text, f"zh-CN {relative_path} must read like an operator manual and include {token!r}")

    def test_priority_docs_have_bilingual_operational_depth(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for relative_path in BILINGUAL_MIN_SECTION_DOCS:
            en_text = (DOCS_SITE / "docs" / relative_path).read_text()
            zh_text = (zh_root / relative_path).read_text()
            en_h2 = [line for line in en_text.splitlines() if line.startswith("## ")]
            zh_h2 = [line for line in zh_text.splitlines() if line.startswith("## ")]
            self.assertGreaterEqual(len(en_h2), 6, f"{relative_path} needs enough English operator-manual sections")
            self.assertGreaterEqual(len(zh_h2), 6, f"{relative_path} needs enough zh-CN operator-manual sections")

    def test_notification_quick_path_examples_are_chainable(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for root in [DOCS_SITE / "docs", zh_root]:
            text = (root / "user-guide/notifications.md").read_text()
            for token in [
                "channel → template → policy → event → delivery",
                "CHANNEL_ID=\"$(curl -fsS -X POST",
                "TEMPLATE_ID=\"$(curl -fsS -X POST",
                "POLICY_ID=\"$(curl -fsS -X POST",
                "jq -r '.data.id'",
                "secretRefs",
                "supportsTestSend=true",
            ]:
                self.assertIn(token, text, f"{root.relative_to(DOCS_SITE)} notifications guide missing chainable quick-path token {token!r}")
            self.assertNotIn("notification-channel-example", text)
            self.assertNotIn("notification-policy-example", text)

    def test_notification_center_docs_are_template_and_provider_schema_backed(self):
        source_bundle = "\n".join(
            path.read_text()
            for path in [
                ROOT / "crates/tikeo-server/src/http/routes/notifications.rs",
                ROOT / "crates/tikeo-server/src/http/routes/notification_providers.rs",
                ROOT / "crates/tikeo-server/src/http/routes/notification_templates.rs",
                ROOT / "crates/tikeo-server/src/notification.rs",
                ROOT / "crates/tikeo-server/src/notification/provider_templates.rs",
                ROOT / "crates/tikeo-storage/src/migration/notification_center.rs",
            ]
        )
        for token in NOTIFICATION_CENTER_SOURCE_TOKENS:
            self.assertIn(token, source_bundle, f"notification center source missing {token!r}")

        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for root in [DOCS_SITE / "docs", zh_root]:
            combined = "\n".join(
                (root / relative_path).read_text()
                for relative_path in [
                    "reference/notification-center.md",
                    "user-guide/notifications.md",
                ]
            )
            for token in NOTIFICATION_CENTER_DOC_TOKENS:
                self.assertIn(token, combined, f"{root.relative_to(DOCS_SITE)} notification docs missing {token!r}")
            self.assertNotIn("vault 路径", combined)
            self.assertNotIn("env 或 vault", combined)
            reference_lines = (root / "reference/notification-center.md").read_text().splitlines()
            table_start = next(
                index
                for index, line in enumerate(reference_lines)
                if line.startswith("| Provider |")
            )
            table_rows = []
            for line in reference_lines[table_start:]:
                if table_rows and not line.strip():
                    break
                self.assertTrue(
                    line.startswith("|"),
                    f"{root.relative_to(DOCS_SITE)} notification provider table is interrupted before all provider rows: {line!r}",
                )
                table_rows.append(line)
            rendered_rows = "\n".join(table_rows)
            for provider in ["webhook", "slack", "dingtalk", "feishu", "wechat_work", "pagerduty", "email", "plugin webhook"]:
                self.assertIn(f"| `{provider}` |" if provider != "plugin webhook" else "| plugin webhook |", rendered_rows)


    def test_quickstart_manual_path_uses_real_bootstrap_fields_and_runnable_sdk_script(self):
        source_bundle = "\n".join(
            path.read_text()
            for path in [
                ROOT / "crates/tikeo-server/src/http/dto.rs",
                ROOT / "crates/tikeo-server/src/http/auth.rs",
                ROOT / "sdks/nodejs/tikeo/src/management.ts",
                ROOT / "sdks/nodejs/tikeo/src/index.ts",
            ]
        )
        for source_token in [
            "registration_open",
            "AuthSession",
            "token",
            "export * from \"./management.js\"",
            "apiJob",
            "apiTrigger",
        ]:
            self.assertIn(source_token, source_bundle)

        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for root in [DOCS_SITE / "docs", zh_root]:
            text = (root / "getting-started/quickstart.md").read_text()
            self.assertIn("data.registrationOpen", text, f"{root.relative_to(DOCS_SITE)} quickstart must use real bootstrap status field")
            self.assertNotIn("bootstrapRequired", text, f"{root.relative_to(DOCS_SITE)} quickstart must not document nonexistent bootstrapRequired field")
            self.assertIn('TOKEN="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/bootstrap/register', text)
            self.assertIn("jq -r .data.token", text)
            self.assertIn("cat >tikeo-quickstart-trigger.ts", text)
            self.assertIn('from "./sdks/nodejs/tikeo/src/index"', text)
            self.assertIn("bun tikeo-quickstart-trigger.ts", text)
            self.assertNotIn("/tmp/tikeo-quickstart-trigger.ts", text)

    def test_operator_grade_docs_are_not_readme_rehash(self):
        """Critical docs must be deep enough to install, configure, connect SDKs, deploy, and verify from implementation-anchored instructions."""
        critical = {
            "index.md": {
                "min_words": 900,
                "tokens": [
                    "Documentation map",
                    "Reader outcome",
                    "Architecture boundary",
                    "Evidence-first evaluation",
                    "Implementation anchors",
                ],
            },
            "getting-started/installation.md": {
                "min_words": 950,
                "tokens": [
                    "Toolchain matrix",
                    "Version baselines",
                    "Repository surfaces",
                    "First-time bootstrap",
                    "Verification commands",
                ],
            },
            "getting-started/quickstart.md": {
                "min_words": 1300,
                "tokens": [
                    "Phase 0",
                    "Bootstrap the first Owner",
                    "Create an app-scoped SDK API key",
                    "TIKEO_WORKER_CONNECT=1",
                    "management-trigger-e2e-smoke.sh",
                    "Acceptance evidence",
                ],
            },
            "reference/configuration.md": {
                "min_words": 1800,
                "tokens": [
                    "Complete default-value table",
                    "storage.timestamp_offset",
                    "cluster.transport_token",
                    "transport_security.worker_tunnel.client_ca_path",
                    "observability.tracing.otlp_endpoint",
                    "TIKEO__AUTH__OIDC__ISSUER_URL",
                    "Worker SDK defaults",
                ],
            },
            "sdks/rust.md": {
                "min_words": 1000,
                "tokens": [
                    "Dependency coordinates",
                    "WorkerConfig defaults",
                    "Minimal Worker",
                    "Management client credentials",
                    "Live verification runbook",
                    "sdks/rust/tikeo/src/config.rs",
                ],
            },
            "sdks/go.md": {
                "min_words": 1000,
                "tokens": [
                    "Dependency coordinates",
                    "WorkerConfig defaults",
                    "Minimal Worker",
                    "Management client credentials",
                    "Live verification runbook",
                    "sdks/go/tikeo/config.go",
                ],
            },
            "sdks/java-spring-boot.md": {
                "min_words": 1300,
                "tokens": [
                    "Dependency coordinates",
                    "Spring Boot property defaults",
                    "@TikeoProcessor",
                    "Management client credentials",
                    "Live verification runbook",
                    "sdks/java/settings.gradle.kts",
                ],
            },
            "sdks/python.md": {
                "min_words": 1000,
                "tokens": [
                    "Dependency coordinates",
                    "WorkerConfig defaults",
                    "Minimal Worker",
                    "Management client credentials",
                    "Live verification runbook",
                    "sdks/python/tikeo/pyproject.toml",
                ],
            },
            "sdks/nodejs.md": {
                "min_words": 1000,
                "tokens": [
                    "Dependency coordinates",
                    "WorkerConfig defaults",
                    "Minimal Worker",
                    "Management client credentials",
                    "Live verification runbook",
                    "sdks/nodejs/tikeo/src/config.ts",
                ],
            },
        }
        for relative_path, rule in critical.items():
            text = (DOCS_SITE / "docs" / relative_path).read_text()
            words = [word for word in re.split(r"\s+", text) if word.strip()]
            self.assertGreaterEqual(len(words), rule["min_words"], f"critical doc is still too shallow: {relative_path}")
            for token in rule["tokens"]:
                self.assertIn(token, text, f"{relative_path} missing operator-grade token {token!r}")

    def test_zh_operator_grade_docs_mirror_critical_depth(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        critical = {
            "index.md": ["文档地图", "阅读结果", "架构边界", "证据优先验收"],
            "getting-started/installation.md": ["工具链矩阵", "版本基线", "仓库工程面", "首次初始化 Owner"],
            "getting-started/quickstart.md": ["阶段 0", "创建首个 Owner", "创建应用级 SDK API Key", "TIKEO_WORKER_CONNECT=1", "验收证据"],
            "reference/configuration.md": ["完整默认值表", "storage.timestamp_offset", "cluster.transport_token", "Worker SDK 默认值"],
            "sdks/rust.md": ["依赖坐标", "WorkerConfig 默认值", "最小 Worker", "管理客户端凭证", "现场验收 runbook"],
            "sdks/go.md": ["依赖坐标", "WorkerConfig 默认值", "最小 Worker", "管理客户端凭证", "现场验收 runbook"],
            "sdks/java-spring-boot.md": ["依赖坐标", "Spring Boot 属性默认值", "@TikeoProcessor", "管理客户端凭证", "现场验收 runbook"],
            "sdks/python.md": ["依赖坐标", "WorkerConfig 默认值", "最小 Worker", "管理客户端凭证", "现场验收 runbook"],
            "sdks/nodejs.md": ["依赖坐标", "WorkerConfig 默认值", "最小 Worker", "管理客户端凭证", "现场验收 runbook"],
        }
        for relative_path, tokens in critical.items():
            text = (zh_root / relative_path).read_text()
            cjk_chars = re.findall(r"[\u4e00-\u9fff]", text)
            self.assertGreaterEqual(len(cjk_chars), 900, f"zh critical doc is still too shallow: {relative_path}")
            for token in tokens:
                self.assertIn(token, text, f"zh {relative_path} missing operator-grade token {token!r}")


    def test_docs_code_blocks_have_loaded_highlight_languages(self):
        config = (DOCS_SITE / "docusaurus.config.ts").read_text()
        loaded_languages = set(re.findall(r"'([a-z0-9-]+)'", config))
        default_languages = {
            "bash", "diff", "html", "javascript", "jsx", "json", "markdown", "mdx",
            "text", "typescript", "tsx", "css", "mermaid", "xml",
        }
        allowed = default_languages | loaded_languages
        doc_roots = [
            DOCS_SITE / "docs",
            DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current",
        ]
        for root in doc_roots:
            for path in root.rglob("*.md"):
                in_fence = False
                for line_no, line in enumerate(path.read_text().splitlines(), 1):
                    stripped = line.strip()
                    if not stripped.startswith("```"):
                        continue
                    if not in_fence:
                        lang = stripped[3:].strip().split()[0] if stripped[3:].strip() else ""
                        self.assertTrue(lang, f"code block lacks highlight language: {path.relative_to(DOCS_SITE)}:{line_no}")
                        self.assertIn(lang, allowed, f"code block language is not loaded by Prism: {path.relative_to(DOCS_SITE)}:{line_no} {lang!r}")
                        in_fence = True
                    else:
                        in_fence = False



    def test_non_java_sdk_parity_is_real_code_not_stale_report_state(self):
        source_expectations = {
            "go": (
                [
                    ROOT / "sdks/go/tikeo/grpc_client.go",
                    ROOT / "sdks/go/tikeo/management.go",
                    ROOT / "sdks/go/tikeo/task_logging.go",
                    ROOT / "sdks/go/tikeo/script_sandbox_tools.go",
                    ROOT / "sdks/go/tikeo/client_test.go",
                ],
                ["ConnectGRPC", "OpenTunnel", "NewManagementClient", "BroadcastAPITrigger", "TaskSlogHandler", "SandboxToolResolver"],
            ),
            "python": (
                [
                    ROOT / "sdks/python/tikeo/src/tikeo/client.py",
                    ROOT / "sdks/python/tikeo/src/tikeo/management.py",
                    ROOT / "sdks/python/tikeo/src/tikeo/task_logging.py",
                    ROOT / "sdks/python/tikeo/src/tikeo/sandbox_tools.py",
                    ROOT / "sdks/python/tikeo/tests/test_sdk.py",
                ],
                ["WorkerTunnelServiceStub", "ManagementClient", "broadcast_api_trigger", "install_task_log_handler", "SandboxToolResolver"],
            ),
            "nodejs": (
                [
                    ROOT / "sdks/nodejs/tikeo/src/client.ts",
                    ROOT / "sdks/nodejs/tikeo/src/management.ts",
                    ROOT / "sdks/nodejs/tikeo/src/taskLogging.ts",
                    ROOT / "sdks/nodejs/tikeo/src/script/sandboxTools.ts",
                    ROOT / "sdks/nodejs/tikeo/tests/sdk.test.ts",
                ],
                ["WorkerTunnelService", "ManagementClient", "broadcastApiTrigger", "installConsoleTaskLogBridge", "SandboxToolResolver"],
            ),
        }
        for language, (paths, tokens) in source_expectations.items():
            combined = "\n".join(path.read_text() for path in paths)
            for token in tokens:
                self.assertIn(token, combined, f"{language} SDK parity source missing {token!r}")

        report = (ROOT / "design/reports/feature-coverage-competitive-checklist.md").read_text()
        self.assertIn("Go/Python/Node SDK | ✅ 已覆盖", report)
        self.assertIn("38** | **38", report)
        self.assertNotIn("Python/Node 未形成完整 SDK", report)
        self.assertNotIn("剩余为非 Java SDK parity", report)



    def test_release_acceptance_packets_are_indexed_and_evidence_backed(self):
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        versions = {
            "v0.3.10": [
                "7b85b2cc",
                "in-place migration apply",
                "tikeo-migrate-0.3.10-x86_64-unknown-linux-gnu.tar.gz",
                "release-readiness-evidence.sh",
                "kind-raft-ha-e2e-20260622.md",
                "TIKEO_CROSS_SOAK_SECONDS=120",
                ".github/workflows/release-candidate-worker-soak.yml",
                "scripts/cloud-raft-ha-acceptance.sh",
            ],
            "v0.3.9": [
                "ee895ba7",
                "affb4605",
                "kind-raft-ha-e2e-20260622.md",
                "TIKEO_CROSS_SOAK_SECONDS=120",
                ".github/workflows/release-candidate-worker-soak.yml",
                "cross-language-worker-soak",
                "cross-language-workers-20260622T065243Z-596956",
                "scripts/cloud-raft-ha-acceptance.sh",
            ],
        }
        for version, tokens in versions.items():
            pages = [
                DOCS_SITE / f"docs/development/release-acceptance-packet-{version}.md",
                zh_root / f"development/release-acceptance-packet-{version}.md",
            ]
            for path in pages:
                self.assertTrue(path.exists(), f"missing release acceptance packet: {path}")
                text = path.read_text()
                for token in [version, "31", *tokens]:
                    self.assertIn(token, text, f"{path.relative_to(DOCS_SITE)} missing {token!r}")
                self.assertNotIn("../../../design/reports", text)

        readme = (ROOT / "README.md").read_text()
        readme_zh = (ROOT / "README.zh-CN.md").read_text()
        sidebars = (DOCS_SITE / "sidebars.ts").read_text()
        search_index = (DOCS_SITE / "static/search-index.json").read_text()
        llms = (DOCS_SITE / "static/llms.txt").read_text() + (DOCS_SITE / "static/llms-full.txt").read_text()
        for route in [
            "/docs/development/release-acceptance-packet-v0.3.10",
            "/zh-CN/docs/development/release-acceptance-packet-v0.3.10",
            "/docs/development/release-acceptance-packet-v0.3.9",
            "/zh-CN/docs/development/release-acceptance-packet-v0.3.9",
        ]:
            self.assertIn(route, readme + readme_zh + search_index + llms)
        self.assertIn("development/release-acceptance-packet-v0.3.10", sidebars)
        self.assertIn("development/release-acceptance-packet-v0.3.9", sidebars)

    def test_cross_language_worker_soak_docs_are_script_backed(self):
        script = (ROOT / "deploy/smoke/cross-language-worker-parity-smoke.sh").read_text()
        for token in [
            "TIKEO_CROSS_SOAK_SECONDS",
            "TIKEO_CROSS_SOAK_INTERVAL_SECONDS",
            "SOAK_METRICS_JSONL",
            "SOAK_JSON",
            "cross-language-soak",
            "queuePending",
            "outboxPending",
            "workersOnline",
        ]:
            self.assertIn(token, script)

        readme = (ROOT / "README.md").read_text()
        readme_zh = (ROOT / "README.zh-CN.md").read_text()
        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        docs = [
            readme,
            readme_zh,
            (DOCS_SITE / "docs/development/product-readiness-acceptance.md").read_text(),
            (zh_root / "development/product-readiness-acceptance.md").read_text(),
        ]
        for doc in docs:
            for token in [
                "TIKEO_CROSS_SOAK_SECONDS=120",
                "deploy/smoke/cross-language-worker-parity-smoke.sh",
                ".github/workflows/release-candidate-worker-soak.yml",
                "cross-language-worker-soak",
                "*-soak-summary.json",
                "*-soak-summary.csv",
                "*-soak-metrics.jsonl",
            ]:
                self.assertIn(token, doc)
        self.assertIn("failed=0", readme)
        self.assertIn("outboxPending", readme + readme_zh)

    def test_cloud_ha_acceptance_runbook_is_script_backed_and_read_only(self):
        script = (ROOT / "scripts/cloud-raft-ha-acceptance.sh").read_text()
        for token in [
            "TIKEO_CLOUD_HA_SERVER_URL",
            "TIKEO_CLOUD_HA_EXPECTED_REPLICAS",
            "TIKEO_CLOUD_HA_WORKER_TUNNEL_HOST",
            "TIKEO_CLOUD_HA_MUTATING",
            "/api/v1/cluster/diagnostics",
            "/api/v1/metrics/summary",
            "/api/v1/dispatch-queue/stream",
            "summary.json",
            "REPORT.md",
            "kubectl-pods.txt",
            "worker-tunnel-tcp.json",
        ]:
            self.assertIn(token, script)
        self.assertIn("mutating cloud HA drills are intentionally disabled", script)
        self.assertNotIn("kubectl delete", script)
        self.assertNotIn("curl -X POST", script)

        zh_root = DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"
        for root in [DOCS_SITE / "docs", zh_root]:
            text = (root / "deployment/server-ha.md").read_text()
            for token in [
                "scripts/cloud-raft-ha-acceptance.sh",
                "TIKEO_CLOUD_HA_SERVER_URL",
                "TIKEO_CLOUD_HA_EXPECTED_REPLICAS",
                "TIKEO_CLOUD_HA_WORKER_TUNNEL_HOST",
                "REPORT.md",
                "summary.json",
                "text/event-stream",
                "shardOwnership.active > 0",
            ]:
                self.assertIn(token, text, f"{root.relative_to(DOCS_SITE)} server-ha missing {token!r}")

    def test_docs_links_indexes_and_generated_surfaces_stay_consistent(self):
        self.assertFalse((DOCS_SITE / "reports").exists(), "validation reports must stay outside docs site root")
        sidebars = (DOCS_SITE / "sidebars.ts").read_text()
        search_index = (DOCS_SITE / "static/search-index.json").read_text()
        llms = (DOCS_SITE / "static/llms.txt").read_text() + (DOCS_SITE / "static/llms-full.txt").read_text()
        for route in [
            "/docs/deployment/server-ha",
            "/zh-CN/docs/deployment/server-ha",
            "/docs/integrations/migrating-from-legacy-schedulers",
            "/zh-CN/docs/integrations/migrating-from-legacy-schedulers",
            "/docs/development/product-readiness-acceptance",
            "/zh-CN/docs/development/product-readiness-acceptance",
            "/docs/development/release-acceptance-packet-v0.3.10",
            "/zh-CN/docs/development/release-acceptance-packet-v0.3.10",
            "/docs/development/release-acceptance-packet-v0.3.9",
            "/zh-CN/docs/development/release-acceptance-packet-v0.3.9",
        ]:
            self.assertIn(route, search_index + llms, f"search/LLM entrypoints missing {route}")
        for doc_id in [
            "deployment/server-ha",
            "integrations/migrating-from-legacy-schedulers",
            "development/product-readiness-acceptance",
            "development/release-acceptance-packet-v0.3.10",
            "development/release-acceptance-packet-v0.3.9",
        ]:
            self.assertIn(doc_id, sidebars)
        for root in [DOCS_SITE / "docs", DOCS_SITE / "i18n/zh-CN/docusaurus-plugin-content-docs/current"]:
            configuration = (root / "reference/configuration.md").read_text()
            product = (root / "development/product-readiness-acceptance.md").read_text()
            heading_slugs = set()
            for heading in re.findall(r"^#+\s+(.+)$", configuration, flags=re.MULTILINE):
                slug = re.sub(r"[^\w\u4e00-\u9fff -]", "", heading.strip().lower())
                slug = re.sub(r"\s+", "-", slug).strip("-")
                heading_slugs.add(slug)
            anchors = re.findall(r"reference/configuration#([^\)\s]+)", product)
            for anchor in anchors:
                self.assertIn(anchor.lower(), heading_slugs, f"broken configuration anchor {anchor!r} in {root.relative_to(DOCS_SITE)} readiness doc")


if __name__ == "__main__":
    unittest.main()
