from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github/workflows"
CI = (WORKFLOWS / "ci.yml").read_text()
GITHUB_RELEASE = (WORKFLOWS / "release-github-assets.yml").read_text()
DOCKER_SERVER = (WORKFLOWS / "publish-docker-server.yml").read_text()
DOCKER_WEB = (WORKFLOWS / "publish-docker-web.yml").read_text()
JAVA_SDK = (WORKFLOWS / "publish-java-sdk.yml").read_text()
RUST_SDK = (WORKFLOWS / "publish-rust-sdk.yml").read_text()
GO_SDK = (WORKFLOWS / "publish-go-sdk.yml").read_text()


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
        }
        for job, expected_name in expected_job_names.items():
            self.assertIn(expected_name, workflow_job_block(CI, job))

        self.assertNotIn("  java-sdk:", CI)
        self.assertNotIn("  java-demos:", CI)
        self.assertNotIn("  rust-sdk:", CI)
        self.assertNotIn("  rust-demo:", CI)
        self.assertNotIn("  go-deploy-tools:", CI)
        self.assertNotIn("  cross-language-smoke:", CI)

    def test_docker_validation_is_split_and_cached(self):
        self.assertNotIn("  docker-build:", CI)
        self.assertIn("  other-docker-build-server:", CI)
        self.assertIn("  other-docker-build-web:", CI)

        server_job = workflow_job_block(CI, "other-docker-build-server")
        web_job = workflow_job_block(CI, "other-docker-build-web")
        self.assertIn("name: Other / Docker build validation / server", server_job)
        self.assertIn("name: Other / Docker build validation / web", web_job)
        self.assertIn("file: Dockerfile", server_job)
        self.assertIn("context: .", server_job)
        self.assertIn("file: web/Dockerfile", web_job)
        self.assertIn("context: web", web_job)
        for job_block in [server_job, web_job]:
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

    def test_ci_runs_docs_site_verification(self):
        docs_job = workflow_job_block(CI, "docs-site")
        self.assertIn("name: Docs site", docs_job)
        self.assertIn("needs: workflow-policy", docs_job)
        self.assertIn("uses: oven-sh/setup-bun@v2", docs_job)
        self.assertIn("bun-version: latest", docs_job)
        self.assertIn("python3 .github/tests/docs_site_contract_test.py", docs_job)
        self.assertIn("working-directory: website", docs_job)
        self.assertIn("bun install --frozen-lockfile", docs_job)
        self.assertIn("bun run docs:typecheck", docs_job)
        self.assertIn("bun run docs:build", docs_job)

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
        self.assertIn("config", GITHUB_RELEASE)
        self.assertIn("softprops/action-gh-release", GITHUB_RELEASE)
        self.assertIn("workflow_dispatch", GITHUB_RELEASE)
        self.assertNotIn("docker/login-action", GITHUB_RELEASE)

    def test_github_release_has_bilingual_changelog_body(self):
        self.assertIn("Generate bilingual changelog", GITHUB_RELEASE)
        self.assertIn("release-notes.md", GITHUB_RELEASE)
        self.assertIn("echo \"## English\"", GITHUB_RELEASE)
        self.assertIn("echo \"## 中文\"", GITHUB_RELEASE)
        self.assertLess(GITHUB_RELEASE.index("echo \"## English\""), GITHUB_RELEASE.index("echo \"## 中文\""))
        self.assertIn("git log", GITHUB_RELEASE)
        self.assertIn("body_path: release-notes.md", GITHUB_RELEASE)

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
