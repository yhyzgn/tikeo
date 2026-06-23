from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github/workflows"
CI = (WORKFLOWS / "ci.yml").read_text()
GITHUB_RELEASE = (WORKFLOWS / "release-github-assets.yml").read_text()
MIGRATE_CLI = (WORKFLOWS / "build-migrate-cli.yml").read_text()
DOCKER_SERVER = (WORKFLOWS / "publish-docker-server.yml").read_text()
DOCKER_WEB = (WORKFLOWS / "publish-docker-web.yml").read_text()
DOCKER_DOCS = (WORKFLOWS / "publish-docker-docs.yml").read_text()
JAVA_SDK = (WORKFLOWS / "publish-java-sdk.yml").read_text()
RUST_SDK = (WORKFLOWS / "publish-rust-sdk.yml").read_text()
GO_SDK = (WORKFLOWS / "publish-go-sdk.yml").read_text()
RC_WORKER_SOAK = (WORKFLOWS / "release-candidate-worker-soak.yml").read_text()


def workflow_job_block(workflow_text: str, job: str) -> str:
    match = re.search(rf"(?ms)^  {re.escape(job)}:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)", workflow_text)
    if not match:
        raise AssertionError(f"job not found: {job}")
    return match.group("body")


class WorkflowContractTest(unittest.TestCase):
    def test_ci_validates_server_web_sdks_and_docker_without_publish(self):
        self.assertIn("cargo fmt --all -- --check", CI)
        self.assertIn("cargo clippy --workspace --all-targets --all-features", CI)
        self.assertNotIn("cargo clippy --workspace --all-targets --all-features -- -D warnings", CI)
        self.assertIn("bun run build", CI)
        self.assertIn("./gradlew test", CI)
        self.assertIn("Test Spring Boot 2 worker demo", CI)
        self.assertIn("cargo test --workspace --all-features -- --test-threads=1", CI)
        self.assertIn("cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features", CI)
        self.assertIn("cargo test --manifest-path examples/rust/worker-demo/Cargo.toml --all-features", CI)
        self.assertIn("Test Go worker demo", CI)
        self.assertIn("Build server image without push", CI)
        self.assertIn("Build web image without push", CI)
        self.assertIn("Build docs image without push", CI)
        self.assertIn("push: false", CI)
        self.assertNotIn("docker/login-action", CI)
        self.assertNotIn("softprops/action-gh-release", CI)

    def test_ci_jobs_are_grouped_by_runtime_surface(self):
        expected_job_names = {
            "server": "name: Server",
            "web": "name: Web",
            "docs-site": "name: Docs site",
            "java-sdk-demo": "name: Java SDK + demo",
            "rust-sdk-demo": "name: Rust SDK + demo",
            "go-sdk-demo": "name: Go SDK + demo",
            "python-sdk-demo": "name: Python SDK + demo",
            "nodejs-sdk-demo": "name: Node.js SDK + demo",
            "other-deploy-tools": "name: Other / deploy tooling",
            "other-cross-language-smoke": "name: Other / cross-language worker smoke",
            "other-docker-build-server": "name: Other / Docker build validation / server",
            "other-docker-build-web": "name: Other / Docker build validation / web",
            "other-docker-build-docs": "name: Other / Docker build validation / docs",
        }
        for job, expected_name in expected_job_names.items():
            self.assertIn(expected_name, workflow_job_block(CI, job))

        self.assertNotIn("  java-sdk:", CI)
        self.assertNotIn("  java-demos:", CI)
        self.assertNotIn("  rust-sdk:", CI)
        self.assertNotIn("  rust-demo:", CI)
        self.assertNotIn("  go-deploy-tools:", CI)
        self.assertNotIn("  cross-language-smoke:", CI)


    def test_cross_language_job_runs_management_trigger_e2e_smoke(self):
        smoke_job = workflow_job_block(CI, "other-cross-language-smoke")
        self.assertIn("Run Management API trigger e2e smoke", smoke_job)
        self.assertIn("TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER", smoke_job)
        self.assertIn("scripts/management-trigger-e2e-smoke.sh", smoke_job)
        self.assertIn("management-trigger-e2e-smoke", smoke_job)
        self.assertIn(".dev/reports/management-trigger-e2e-*/*", smoke_job)
        self.assertLess(
            smoke_job.index("Run cross-language worker parity smoke"),
            smoke_job.index("Run Management API trigger e2e smoke"),
        )

    def test_docker_validation_is_split_and_cached(self):
        self.assertNotIn("  docker-build:", CI)
        self.assertIn("  other-docker-build-server:", CI)
        self.assertIn("  other-docker-build-web:", CI)
        self.assertIn("  other-docker-build-docs:", CI)

        server_job = workflow_job_block(CI, "other-docker-build-server")
        web_job = workflow_job_block(CI, "other-docker-build-web")
        docs_job = workflow_job_block(CI, "other-docker-build-docs")
        self.assertIn("name: Other / Docker build validation / server", server_job)
        self.assertIn("name: Other / Docker build validation / web", web_job)
        self.assertIn("name: Other / Docker build validation / docs", docs_job)
        self.assertIn("file: Dockerfile", server_job)
        self.assertIn("context: .", server_job)
        self.assertIn("file: web/Dockerfile", web_job)
        self.assertIn("context: web", web_job)
        self.assertIn("file: docs/Dockerfile", docs_job)
        self.assertIn("context: docs", docs_job)
        self.assertIn("needs:", docs_job)
        self.assertIn("docs-site", docs_job)
        for job_block in [server_job, web_job, docs_job]:
            self.assertIn("cache-from: type=gha", job_block)
            self.assertIn("cache-to: type=gha,mode=max", job_block)
            self.assertIn("push: false", job_block)
            self.assertIn("load: false", job_block)
            self.assertIn("needs:", job_block)
            self.assertIn("other-cross-language-smoke", job_block)

    def test_ci_rejects_node20_or_older_action_runtimes_before_other_jobs(self):
        self.assertTrue((ROOT / "scripts/verify-github-actions-node-runtime.py").exists())
        self.assertIn("workflow-policy:", CI)
        policy_job = CI.split("  workflow-policy:", 1)[1].split("\n  server:", 1)[0]
        self.assertIn("Reject deprecated GitHub Actions Node runtimes", policy_job)
        self.assertIn("verify-github-actions-node-runtime.py", policy_job)
        self.assertIn("--min-node-major 24", policy_job)
        self.assertNotIn("uses:", policy_job)
        self.assertIn(r"^\s*-?\s*uses\s*:", (ROOT / "scripts/verify-github-actions-node-runtime.py").read_text())

        for job in [
            "server",
            "web",
            "docs-site",
            "java-sdk-demo",
            "go-sdk-demo",
            "other-deploy-tools",
            "rust-sdk-demo",
            "python-sdk-demo",
            "nodejs-sdk-demo",
        ]:
            job_block = workflow_job_block(CI, job)
            self.assertIn("needs: workflow-policy", job_block)


    def test_workflow_policy_runs_repository_contract_tests(self):
        policy_job = CI.split("  workflow-policy:", 1)[1].split("\n  server:", 1)[0]
        self.assertIn("Validate repository contract tests", policy_job)
        self.assertIn("python3 .github/tests/workflow_contract_test.py", policy_job)
        self.assertIn("python3 .github/tests/management_smoke_contract_test.py", policy_job)
        self.assertLess(
            policy_job.index("Reject deprecated GitHub Actions Node runtimes"),
            policy_job.index("Validate repository contract tests"),
        )

    def test_ci_runs_docs_site_verification(self):
        docs_job = workflow_job_block(CI, "docs-site")
        self.assertIn("name: Docs site", docs_job)
        self.assertIn("needs: workflow-policy", docs_job)
        self.assertIn("uses: oven-sh/setup-bun@v2", docs_job)
        self.assertIn("bun-version: latest", docs_job)
        self.assertIn("python3 .github/tests/docs_site_contract_test.py", docs_job)
        self.assertIn("working-directory: docs", docs_job)
        self.assertNotIn("working-directory: website", docs_job)
        self.assertIn("bun install --frozen-lockfile", docs_job)
        self.assertIn("bun run docs:typecheck", docs_job)
        self.assertIn("bun run docs:build", docs_job)
        self.assertIn("Validate docs SEO output", docs_job)
        self.assertIn("python3 .github/tests/docs_seo_contract_test.py", docs_job)
        self.assertLess(docs_job.index("bun run docs:build"), docs_job.index("docs_seo_contract_test.py"))

    def test_ci_enforces_source_size_before_runtime_jobs(self):
        self.assertTrue((ROOT / "scripts/check-source-size.py").exists())
        self.assertIn("workflow-policy:", CI)
        policy_job = CI.split("  workflow-policy:", 1)[1].split("\n  server:", 1)[0]
        self.assertIn("Enforce source file size budget", policy_job)
        self.assertIn("python3 scripts/check-source-size.py", policy_job)
        self.assertLess(
            policy_job.index("Enforce source file size budget"),
            CI.index("  server:"),
        )


    def test_python_and_node_sdk_demo_jobs_are_real_release_gates(self):
        python_job = workflow_job_block(CI, "python-sdk-demo")
        self.assertIn("python -m pip install -e sdks/python/tikeo[test]", python_job)
        self.assertIn("python -m pip install -e examples/python/worker-demo[test]", python_job)
        self.assertIn("python -m pytest sdks/python/tikeo/tests examples/python/worker-demo/tests -q", python_job)
        self.assertIn("python -m tikeo_python_worker_demo", python_job)
        self.assertNotIn("Planned placeholder", python_job)

        node_job = workflow_job_block(CI, "nodejs-sdk-demo")
        self.assertIn("working-directory: sdks/nodejs/tikeo", node_job)
        self.assertIn("bun test", node_job)
        self.assertIn("bun run build", node_job)
        self.assertIn("working-directory: examples/nodejs/worker-demo", node_job)
        self.assertIn("bun start", node_job)
        self.assertNotIn("Planned placeholder", node_job)

    def test_legacy_aggregate_release_workflow_is_removed(self):
        self.assertFalse((WORKFLOWS / "release.yml").exists())

    def test_github_release_assets_are_independent(self):
        for target in ["x86_64-unknown-linux-gnu", "x86_64-apple-darwin", "aarch64-apple-darwin", "x86_64-pc-windows-msvc"]:
            self.assertIn(target, GITHUB_RELEASE)
        self.assertIn("tikeo-web-dist", GITHUB_RELEASE)
        self.assertIn("deploy-assets", GITHUB_RELEASE)
        self.assertIn("terraform-provider-tikeo", GITHUB_RELEASE)
        self.assertIn("helm package deploy/helm/tikeo", GITHUB_RELEASE)
        self.assertIn("migrate-cli-binaries", GITHUB_RELEASE)
        self.assertIn("tikeo-migrate-${VERSION}-${{ matrix.target }}", GITHUB_RELEASE)
        self.assertIn("--bin tikeo-migrate", GITHUB_RELEASE)
        self.assertIn("needs: [server-binaries, migrate-cli-binaries, web-dist, deploy-assets]", GITHUB_RELEASE)
        self.assertIn("config", GITHUB_RELEASE)
        self.assertIn("softprops/action-gh-release", GITHUB_RELEASE)
        self.assertIn("workflow_dispatch", GITHUB_RELEASE)
        self.assertNotIn("docker/login-action", GITHUB_RELEASE)


    def test_release_candidate_worker_soak_is_manual_evidence_gate(self):
        self.assertIn("Release candidate / cross-language Worker soak", RC_WORKER_SOAK)
        self.assertIn("workflow_dispatch", RC_WORKER_SOAK)
        for token in [
            "ref:",
            "soak_seconds:",
            "default: '120'",
            "soak_interval_seconds:",
            "default: '10'",
            "rebuild_server:",
            "skip_web:",
            "cancel-in-progress: false",
            "timeout-minutes: 45",
        ]:
            self.assertIn(token, RC_WORKER_SOAK)

        soak_job = workflow_job_block(RC_WORKER_SOAK, "soak")
        for token in [
            "actions/checkout@v6",
            "dtolnay/rust-toolchain@stable",
            "actions/setup-java@v5",
            "actions/setup-go@v6",
            "actions/setup-python@v6",
            "oven-sh/setup-bun@v2",
            "bun install --frozen-lockfile",
            "python -m pip install -e sdks/python/tikeo",
            "python -m pip install -e examples/python/worker-demo",
            "TIKEO_CROSS_SKIP_WEB",
            "TIKEO_CROSS_REBUILD_SERVER",
            "TIKEO_CROSS_SOAK_SECONDS",
            "TIKEO_CROSS_SOAK_INTERVAL_SECONDS",
            "deploy/smoke/cross-language-worker-parity-smoke.sh",
            "GITHUB_STEP_SUMMARY",
            "cross-language-worker-soak",
            "actions/upload-artifact@v6",
            ".dev/reports/cross-language-workers-*/*",
        ]:
            self.assertIn(token, soak_job)
        self.assertNotIn("softprops/action-gh-release", RC_WORKER_SOAK)
        self.assertNotIn("docker/login-action", RC_WORKER_SOAK)
        self.assertNotIn("contents: write", RC_WORKER_SOAK)

    def test_migration_cli_binary_ci_is_separate_and_artifact_only(self):
        self.assertIn("CI / Migration CLI binaries", MIGRATE_CLI)
        self.assertIn("cargo test -p tikeo-migrate --locked --target", MIGRATE_CLI)
        self.assertIn("cargo build -p tikeo-migrate --release --locked --bin tikeo-migrate", MIGRATE_CLI)
        self.assertIn("actions/upload-artifact@v6", MIGRATE_CLI)
        self.assertIn("tikeo-migrate-${VERSION}-${{ matrix.target }}", MIGRATE_CLI)
        self.assertIn("x86_64-unknown-linux-gnu", MIGRATE_CLI)
        self.assertIn("x86_64-apple-darwin", MIGRATE_CLI)
        self.assertIn("aarch64-apple-darwin", MIGRATE_CLI)
        self.assertIn("x86_64-pc-windows-msvc", MIGRATE_CLI)
        self.assertNotIn("softprops/action-gh-release", MIGRATE_CLI)
        self.assertNotIn("contents: write", MIGRATE_CLI)

    def test_github_release_has_product_release_notes_body(self):
        self.assertIn("Generate product release notes", GITHUB_RELEASE)
        self.assertIn("scripts/generate-release-notes.py", GITHUB_RELEASE)
        self.assertIn("release-notes.md", GITHUB_RELEASE)
        self.assertIn("body_path: release-notes.md", GITHUB_RELEASE)
        self.assertNotIn("Generate bilingual changelog", GITHUB_RELEASE)
        self.assertNotIn("echo \"## English\"", GITHUB_RELEASE)
        self.assertNotIn("echo \"## 中文\"", GITHUB_RELEASE)


    def test_server_release_and_docker_publish_gate_manifest_version_from_tag(self):
        github_server_job = workflow_job_block(GITHUB_RELEASE, "server-binaries")
        docker_server_job = workflow_job_block(DOCKER_SERVER, "publish")
        for job_block in [github_server_job, docker_server_job]:
            self.assertIn("Resolve release version", job_block)
            self.assertIn("RELEASE_TAG", job_block)
            self.assertIn('VERSION="${RELEASE_TAG#v}"', job_block)
            self.assertIn("scripts/check-release-version.py", job_block)
            self.assertIn("Verify release tag commit", job_block)
            self.assertIn("git rev-list -n 1", job_block)
            self.assertNotIn("scripts/set-release-version.py", job_block)
            self.assertNotIn("--scope workspace", job_block)
        self.assertIn("github.event_name == 'workflow_dispatch' && inputs.tag || github.ref_name", docker_server_job)
        self.assertLess(
            github_server_job.index("scripts/check-release-version.py"),
            github_server_job.index("Build server binary"),
        )
        self.assertLess(
            docker_server_job.index("scripts/check-release-version.py"),
            docker_server_job.index("docker/build-push-action@v7"),
        )
        self.assertIn("TIKEO_GIT_TAG=${{ steps.version.outputs.tag }}", docker_server_job)
        self.assertIn("TIKEO_GIT_SHA=${{ steps.buildmeta.outputs.git_sha }}", docker_server_job)
        self.assertIn("TIKEO_BUILD_TIME=${{ steps.buildmeta.outputs.build_time }}", docker_server_job)

    def test_docker_server_manual_dispatch_can_build_from_explicit_ref(self):
        self.assertIn("ref:", DOCKER_SERVER)
        self.assertIn("Git ref to build from. Defaults to the tag input.", DOCKER_SERVER)
        self.assertIn("inputs.ref || inputs.tag", DOCKER_SERVER)

    def test_docker_publish_targets_are_split(self):
        self.assertIn("yhyzgn/tikeo-server", DOCKER_SERVER)
        self.assertIn("docker/login-action", DOCKER_SERVER)
        self.assertIn("docker/build-push-action", DOCKER_SERVER)
        self.assertIn("push: true", DOCKER_SERVER)
        self.assertNotIn("yhyzgn/tikeo-web", DOCKER_SERVER)

        self.assertIn("yhyzgn/tikeo-web", DOCKER_WEB)
        self.assertIn("docker/login-action", DOCKER_WEB)
        self.assertIn("docker/build-push-action", DOCKER_WEB)
        self.assertIn("push: true", DOCKER_WEB)
        self.assertNotIn("yhyzgn/tikeo-server", DOCKER_WEB)

        self.assertIn("yhyzgn/tikeo-docs", DOCKER_DOCS)
        self.assertIn("docker/login-action", DOCKER_DOCS)
        self.assertIn("docker/build-push-action", DOCKER_DOCS)
        self.assertIn("push: true", DOCKER_DOCS)
        self.assertIn("context: docs", DOCKER_DOCS)
        self.assertIn("file: docs/Dockerfile", DOCKER_DOCS)
        self.assertNotIn("yhyzgn/tikeo-web", DOCKER_DOCS)
        self.assertNotIn("yhyzgn/tikeo-server", DOCKER_DOCS)

    def test_docker_publish_tags_are_pull_friendly_release_aliases(self):
        for workflow_text in [DOCKER_SERVER, DOCKER_WEB, DOCKER_DOCS]:
            job_block = workflow_job_block(workflow_text, "publish")
            self.assertIn("Docker metadata", job_block)
            self.assertIn("type=raw,value=${{ steps.version.outputs.tag }}", job_block)
            self.assertIn("type=raw,value=${{ steps.version.outputs.version }}", job_block)
            self.assertIn("type=raw,value=latest", job_block)
            self.assertIn("provenance: false", job_block)
            self.assertIn("sbom: false", job_block)
            self.assertNotIn("!startsWith(github.ref_name, 'v0.')", job_block)

        for workflow_text in [DOCKER_WEB, DOCKER_DOCS]:
            job_block = workflow_job_block(workflow_text, "publish")
            self.assertIn("Resolve image tag", job_block)
            self.assertIn('VERSION="${RELEASE_TAG#v}"', job_block)

    def test_docker_publish_updates_dockerhub_overview(self):
        expected = {
            DOCKER_SERVER: ("yhyzgn/tikeo-server", "dockerhub/overviews/tikeo-server.md"),
            DOCKER_WEB: ("yhyzgn/tikeo-web", "dockerhub/overviews/tikeo-web.md"),
            DOCKER_DOCS: ("yhyzgn/tikeo-docs", "dockerhub/overviews/tikeo-docs.md"),
        }
        for workflow_text, (repository, readme) in expected.items():
            job_block = workflow_job_block(workflow_text, "publish")
            self.assertIn("Update Docker Hub overview", job_block)
            self.assertIn("scripts/update-dockerhub-overview.py", job_block)
            self.assertIn(f"--repository {repository}", job_block)
            self.assertIn(f"--readme {readme}", job_block)
            self.assertIn("DOCKERHUB_USERNAME", job_block)
            self.assertIn("DOCKERHUB_TOKEN", job_block)

    def test_dockerhub_overviews_include_run_and_compose_paths(self):
        for readme in [
            ROOT / "dockerhub/overviews/tikeo-server.md",
            ROOT / "dockerhub/overviews/tikeo-web.md",
            ROOT / "dockerhub/overviews/tikeo-docs.md",
        ]:
            content = readme.read_text()
            self.assertIn("docker run", content)
            self.assertIn("Docker Compose", content)
            self.assertIn("docker compose", content)
            self.assertIn("latest", content)
            self.assertIn("v0.2.12", content)


    def test_release_setup_includes_docs_docker_publish_lane(self):
        setup = (ROOT / ".github/RELEASE_SETUP.md").read_text()
        self.assertIn("Docker docs", setup)
        self.assertIn(".github/workflows/publish-docker-docs.yml", setup)
        self.assertIn("yhyzgn/tikeo-docs", setup)
        self.assertIn("Release candidate Worker soak", setup)
        self.assertIn(".github/workflows/release-candidate-worker-soak.yml", setup)
        self.assertIn("cross-language-worker-soak", setup)
        self.assertIn("Docker server, Docker web, and Docker docs are separate workflows", setup)

    def test_sdk_publish_targets_are_split(self):
        self.assertIn("sdks/java", JAVA_SDK)
        self.assertIn("java-sdk", JAVA_SDK)
        self.assertIn("./gradlew test --no-daemon", JAVA_SDK)
        self.assertIn("publishAndReleaseToMavenCentral", JAVA_SDK)
        self.assertIn("MAVEN_CENTRAL_USERNAME", JAVA_SDK)
        self.assertNotIn("sdks/rust/tikeo", JAVA_SDK)

        self.assertIn("sdks/rust/tikeo", RUST_SDK)
        self.assertIn("rust-sdk", RUST_SDK)
        self.assertIn("cargo package --manifest-path sdks/rust/tikeo/Cargo.toml", RUST_SDK)
        self.assertIn("cargo publish --manifest-path sdks/rust/tikeo/Cargo.toml", RUST_SDK)
        self.assertIn("CRATES_IO_TOKEN", RUST_SDK)
        self.assertNotIn("sdks/java", RUST_SDK)

        self.assertIn("sdks/go/tikeo", GO_SDK)
        self.assertIn("go-sdk", GO_SDK)
        self.assertIn("go test ./... -count=1", GO_SDK)
        self.assertIn("Publish Go module version tag", GO_SDK)
        self.assertIn("sdks/go/tikeo/${RELEASE_TAG}", GO_SDK)
        self.assertIn("softprops/action-gh-release", GO_SDK)
        self.assertNotIn("CRATES_IO_TOKEN", GO_SDK)


if __name__ == "__main__":
    unittest.main()
