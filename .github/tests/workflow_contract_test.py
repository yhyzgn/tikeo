from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github/workflows"
CI = (WORKFLOWS / "ci.yml").read_text()
GITHUB_RELEASE = (WORKFLOWS / "release-github-assets.yml").read_text()
DOCKER_SERVER = (WORKFLOWS / "publish-docker-server.yml").read_text()
DOCKER_WEB = (WORKFLOWS / "publish-docker-web.yml").read_text()
JAVA_SDK = (WORKFLOWS / "publish-java-sdk.yml").read_text()
RUST_SDK = (WORKFLOWS / "publish-rust-sdk.yml").read_text()


class WorkflowContractTest(unittest.TestCase):
    def test_ci_validates_server_web_sdks_and_docker_without_publish(self):
        self.assertIn("cargo fmt --all -- --check", CI)
        self.assertIn("cargo clippy --workspace --all-targets --all-features", CI)
        self.assertIn("bun run build", CI)
        self.assertIn("./gradlew test", CI)
        self.assertIn("cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features", CI)
        self.assertIn("Build server image without push", CI)
        self.assertIn("Build web image without push", CI)
        self.assertIn("push: false", CI)
        self.assertNotIn("docker/login-action", CI)
        self.assertNotIn("softprops/action-gh-release", CI)

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
