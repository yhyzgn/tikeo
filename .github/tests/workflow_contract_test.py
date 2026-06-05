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
        self.assertIn("cargo test --workspace --all-features -- --test-threads=1", CI)
        self.assertIn("cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features", CI)
        self.assertIn("Build server image without push", CI)
        self.assertIn("Build web image without push", CI)
        self.assertIn("push: false", CI)
        self.assertNotIn("docker/login-action", CI)
        self.assertNotIn("softprops/action-gh-release", CI)

    def test_docker_validation_is_split_and_cached(self):
        self.assertNotIn("  docker-build:", CI)
        self.assertIn("  docker-build-server:", CI)
        self.assertIn("  docker-build-web:", CI)

        server_job = workflow_job_block(CI, "docker-build-server")
        web_job = workflow_job_block(CI, "docker-build-web")
        self.assertIn("name: Docker build validation / server", server_job)
        self.assertIn("name: Docker build validation / web", web_job)
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
            self.assertIn("cross-language-smoke", job_block)

    def test_ci_rejects_node20_or_older_action_runtimes_before_other_jobs(self):
        self.assertTrue((ROOT / "scripts/verify-github-actions-node-runtime.py").exists())
        self.assertIn("workflow-policy:", CI)
        policy_job = CI.split("  workflow-policy:", 1)[1].split("\n  server:", 1)[0]
        self.assertIn("Reject deprecated GitHub Actions Node runtimes", policy_job)
        self.assertIn("verify-github-actions-node-runtime.py", policy_job)
        self.assertIn("--min-node-major 24", policy_job)
        self.assertNotIn("uses:", policy_job)
        self.assertIn(r"^\s*-?\s*uses\s*:", (ROOT / "scripts/verify-github-actions-node-runtime.py").read_text())

        for job in ["server", "web", "java-sdk", "java-demos", "go-sdk-demo", "go-deploy-tools", "rust-sdk", "rust-demo"]:
            job_block = workflow_job_block(CI, job)
            self.assertIn("needs: workflow-policy", job_block)

    def test_legacy_aggregate_release_workflow_is_removed(self):
        self.assertFalse((WORKFLOWS / "release.yml").exists())

    def test_github_release_assets_are_independent(self):
        for target in ["x86_64-unknown-linux-gnu", "x86_64-apple-darwin", "aarch64-apple-darwin", "x86_64-pc-windows-msvc"]:
            self.assertIn(target, GITHUB_RELEASE)
        self.assertIn("tikee-web-dist", GITHUB_RELEASE)
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
        self.assertIn("tikee-server", DOCKER_SERVER)
        self.assertIn("docker/login-action", DOCKER_SERVER)
        self.assertIn("docker/build-push-action", DOCKER_SERVER)
        self.assertIn("push: true", DOCKER_SERVER)
        self.assertNotIn("tikee-web", DOCKER_SERVER)

        self.assertIn("tikee-web", DOCKER_WEB)
        self.assertIn("docker/login-action", DOCKER_WEB)
        self.assertIn("docker/build-push-action", DOCKER_WEB)
        self.assertIn("push: true", DOCKER_WEB)
        self.assertNotIn("tikee-server", DOCKER_WEB)

    def test_sdk_publish_targets_are_split(self):
        self.assertIn("sdks/java", JAVA_SDK)
        self.assertIn("java-sdk", JAVA_SDK)
        self.assertIn("./gradlew test jar sourcesJar", JAVA_SDK)
        self.assertNotIn("sdks/rust/tikee", JAVA_SDK)

        self.assertIn("sdks/rust/tikee", RUST_SDK)
        self.assertIn("rust-sdk", RUST_SDK)
        self.assertIn("cargo package --manifest-path sdks/rust/tikee/Cargo.toml", RUST_SDK)
        self.assertNotIn("sdks/java", RUST_SDK)


if __name__ == "__main__":
    unittest.main()
