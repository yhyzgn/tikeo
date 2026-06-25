from pathlib import Path
import re
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


TEXT_SUFFIXES = {".md", ".rs", ".ts", ".tsx", ".js", ".java", ".sh", ".py", ".sql", ".toml", ".yaml", ".yml"}


def tracked_text_files() -> list[Path]:
    output = subprocess.check_output(["git", "ls-files"], cwd=ROOT, text=True)
    paths = []
    for line in output.splitlines():
        path = ROOT / line
        if (
            path.exists()
            and path.suffix in TEXT_SUFFIXES
            and "docs/build" not in line
            and "web/dist" not in line
        ):
            paths.append(path)
    return paths


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


class NoHardcodedRuntimeContractTest(unittest.TestCase):
    def test_smoke_scripts_do_not_resurrect_default_admin_credentials(self):
        forbidden = ["smoke" + "_admin", "Tikeo" + "@2026!"]
        paths = [
            "deploy/smoke/lib/tikeo-smoke-lib.sh",
            "deploy/smoke/cross-language-worker-parity-smoke.sh",
            "deploy/smoke/java-demo-integration-smoke.sh",
            "scripts/dev-integration-seed.sh",
            "scripts/db-seed-api-compat-smoke.sh",
        ]
        offenders = []
        for path in paths:
            text = read(path)
            for token in forbidden:
                if token in text:
                    offenders.append(f"{path}: {token}")
        self.assertEqual([], offenders)
        helper = read("deploy/smoke/lib/tikeo-smoke-lib.sh")
        self.assertIn("registration_open", helper)
        self.assertIn("tikeo_smoke_random_password", helper)
        self.assertIn("missing smoke authentication credentials", helper)
        self.assertIn('[[ -n "$TIKEO_SMOKE_AUTH_TOKEN" ]]', helper)
        self.assertLess(
            helper.index('[[ -n "$TIKEO_SMOKE_AUTH_TOKEN" ]]'),
            helper.index('registration_open="$(curl'),
            "smoke login must reuse an existing bearer token before checking bootstrap/admin credentials",
        )

    def test_tracked_text_does_not_reintroduce_retired_default_admin_credentials(self):
        forbidden = ["smoke" + "_admin", "Tikeo" + "@2026!"]
        offenders: list[str] = []
        for path in tracked_text_files():
            text = path.read_text(encoding="utf-8")
            for token in forbidden:
                if token in text:
                    offenders.append(f"{path.relative_to(ROOT)}: {token}")
        self.assertEqual([], offenders)

    def test_java_sources_do_not_use_static_imports(self):
        offenders: list[str] = []
        for path in tracked_text_files():
            if path.suffix != ".java":
                continue
            for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if line.strip().startswith("import static "):
                    offenders.append(f"{path.relative_to(ROOT)}:{line_no}")
        self.assertEqual([], offenders)

    def test_provider_metadata_does_not_embed_runtime_examples(self):
        server = read("crates/tikeo-server/src/http/routes/notification_providers.rs")
        web = read("web/src/pages/notifications/providerSchema.ts")
        forbidden = [
            "attach_builtin_examples",
            "builtin_example_template",
            "generatedExample",
            "directWebhookUrl",
            "direct-channel-" + "token",
            "hooks" + ".example.com/tikeo",
            "builtin" + "_feishu_job_card_template",
        ]
        offenders = [token for token in forbidden if token in server or token in web]
        self.assertEqual([], offenders)

    def test_schema_migrations_do_not_seed_notification_channel_examples(self):
        migration = read("crates/tikeo-storage/src/migration/notification_center.rs")
        migrator = read("crates/tikeo-storage/src/migration/mod.rs")
        forbidden = [
            "seed_notification_channel_examples",
            "refresh_seed_notification_channel_examples",
            "notification_channel_example_template",
            "NotificationChannelRichExamplesMigration",
            "NotificationChannelDirectCredentialExamplesMigration",
            "NotificationChannelEmailSmtpExamplesMigration",
        ]
        offenders = [token for token in forbidden if token in migration or token in migrator]
        self.assertEqual([], offenders)
        self.assertIn("NotificationChannelExamplesCleanupMigration", migrator)
        self.assertRegex(migration, re.compile(r"DELETE FROM notification_channels WHERE id LIKE 'notification-channel-example-%'"))


if __name__ == "__main__":
    unittest.main()
