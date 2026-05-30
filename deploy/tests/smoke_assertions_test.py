import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ASSERT = ROOT / "deploy" / "smoke" / "assert_tikee_expectations.py"


def run_assertion(kind, payload, *args):
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", suffix=".json", delete=False) as fh:
        json.dump(payload, fh)
        path = fh.name
    try:
        return subprocess.run(
            [sys.executable, str(ASSERT), kind, path, *args],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
    finally:
        Path(path).unlink(missing_ok=True)


class SmokeAssertionsTest(unittest.TestCase):
    def test_workers_require_exactly_one_master_per_domain_and_structured_capabilities(self):
        payload = {
            "data": {
                "items": [
                    {
                        "workerId": "worker-a",
                        "clientInstanceId": "spring-demo-worker-a",
                        "status": "online",
                        "structuredCapabilities": {
                            "tags": ["java", "spring-boot"],
                            "sdkProcessors": ["demo.echo"],
                            "pluginProcessors": [{"type": "sql", "processorNames": ["billing.sql-sync"]}],
                            "scriptRunners": [{"language": "shell", "sandboxBackend": "wasmtime"}],
                        },
                        "master": {"domain": "default/default/local/local", "isMaster": True, "masterWorkerId": "worker-a", "term": 1, "fencingToken": "tok-a"},
                    },
                    {
                        "workerId": "worker-b",
                        "clientInstanceId": "spring-demo-worker-b",
                        "status": "online",
                        "structuredCapabilities": {
                            "tags": ["java", "spring-boot"],
                            "sdkProcessors": ["demo.echo"],
                            "pluginProcessors": [{"type": "sql", "processorNames": ["billing.sql-sync"]}],
                            "scriptRunners": [{"language": "shell", "sandboxBackend": "wasmtime"}],
                        },
                        "master": {"domain": "default/default/local/local", "isMaster": False, "masterWorkerId": "worker-a", "term": 1, "fencingToken": "tok-b"},
                    },
                ]
            }
        }
        result = run_assertion("workers", payload, "--client-instance", "spring-demo-worker-a", "--require-capability", "java", "--require-sdk-processor", "demo.echo", "--require-plugin-processor", "sql:billing.sql-sync", "--require-script-runner", "shell")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)

    def test_workers_fail_when_two_masters_exist_in_one_domain(self):
        payload = {
            "data": {
                "items": [
                    {"workerId": "worker-a", "status": "online", "structuredCapabilities": {}, "master": {"domain": "d", "isMaster": True}},
                    {"workerId": "worker-b", "status": "online", "structuredCapabilities": {}, "master": {"domain": "d", "isMaster": True}},
                ]
            }
        }
        result = run_assertion("workers", payload)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("expected exactly one master", result.stderr)

    def test_instance_and_logs_expect_business_status_and_log_text(self):
        instance = {"data": {"id": "inst-1", "status": "succeeded", "workerId": "worker-a", "logCount": 1}}
        logs = {"data": {"items": [{"message": "[demo.echo] processed payload hello", "workerId": "worker-a"}]}}
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", suffix=".json", delete=False) as lf:
            json.dump(logs, lf)
            logs_path = lf.name
        try:
            result = run_assertion("instance", instance, "--expected-status", "succeeded", "--expected-worker", "worker-a", "--logs-file", logs_path, "--require-log-text", "demo.echo")
            self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        finally:
            Path(logs_path).unlink(missing_ok=True)


if __name__ == "__main__":
    unittest.main()
