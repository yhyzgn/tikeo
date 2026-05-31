from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
CI = (ROOT / ".github/workflows/ci.yml").read_text()
RELEASE = (ROOT / ".github/workflows/release.yml").read_text() if (ROOT / ".github/workflows/release.yml").exists() else ""


class WorkflowContractTest(unittest.TestCase):
    def test_ci_validates_server_web_and_sdks(self):
        self.assertIn("cargo fmt --all -- --check", CI)
        self.assertIn("cargo clippy --workspace --all-targets --all-features", CI)
        self.assertIn("bun run build", CI)
        self.assertIn("./gradlew test", CI)
        self.assertIn("cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features", CI)

    def test_release_builds_cross_platform_archives_and_web_dist(self):
        for target in ["x86_64-unknown-linux-gnu", "x86_64-apple-darwin", "aarch64-apple-darwin", "x86_64-pc-windows-msvc"]:
            self.assertIn(target, RELEASE)
        self.assertIn("tikee-web-dist", RELEASE)
        self.assertIn("config", RELEASE)
        self.assertIn("softprops/action-gh-release", RELEASE)

    def test_release_pushes_docker_hub_images_and_sdk_packages(self):
        self.assertIn("Build server image without push", CI)
        self.assertIn("push: false", CI)
        self.assertIn("docker/login-action", RELEASE)
        self.assertIn("DOCKERHUB_USERNAME", RELEASE)
        self.assertIn("tikee-server", RELEASE)
        self.assertIn("tikee-web", RELEASE)
        self.assertIn("java-sdk", RELEASE)
        self.assertIn("rust-sdk", RELEASE)
        self.assertIn("sdks/java", RELEASE)
        self.assertIn("sdks/rust/tikee", RELEASE)
        self.assertIn("tags:", RELEASE)
        self.assertNotIn("workflow_dispatch", RELEASE)


if __name__ == "__main__":
    unittest.main()
