from pathlib import Path
import stat
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "management-trigger-e2e-smoke.sh"


class ManagementTriggerSmokeContractTest(unittest.TestCase):
    def test_management_trigger_smoke_script_is_repeatable_and_source_backed(self):
        self.assertTrue(SCRIPT.exists(), "scripts/management-trigger-e2e-smoke.sh must exist")
        mode = SCRIPT.stat().st_mode
        self.assertTrue(mode & stat.S_IXUSR, "management trigger smoke script must be executable")
        text = SCRIPT.read_text()

        for token in [
            "set -euo pipefail",
            "trap cleanup EXIT INT TERM",
            "DB_PATH=",
            "[storage.database]",
            "type = \"sqlite\"",
            "path = \"$DB_PATH\"",
            "mode = \"rwc\"",
            "serve --config \"$SERVER_CONFIG\"",
            "tikeo_smoke_wait_for_http server \"$API_URL/readyz\"",
            "POST /api/v1/management/service-accounts",
            "POST /api/v1/management/api-keys",
            "TIKEO_API_KEY",
            "x-tikeo-api-key",
            "ManagementClient",
            "apiJob",
            "apiTrigger",
            "createJob",
            "triggerJob",
            "TIKEO_WORKER_ENDPOINT=\"$WORKER_ENDPOINT\"",
            "TIKEO_WORKER_CONNECT=1",
            "bun start",
            "/api/v1/workers",
            "clientInstanceId",
            "/api/v1/instances/$instance_id",
            "/api/v1/instances/$instance_id/logs",
            "result.success",
            "nodejs demo echo processed",
            "tikeo_smoke_record_case management-sdk-create-trigger",
            "tikeo_smoke_finalize_report",
        ]:
            self.assertIn(token, text, f"smoke script missing required contract token: {token}")

        self.assertNotIn("TIKEO_WORKER_DRY_RUN=1", text)
        self.assertNotIn("TIKEO_WORKER_DRY_RUN=true", text)


if __name__ == "__main__":
    unittest.main()
