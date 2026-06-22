from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
JAVA_DEMO_SCRIPTS = [
    ROOT / "examples/java/spring-boot2-worker-demo/scripts/run-demo-worker.sh",
    ROOT / "examples/java/spring-boot3-worker-demo/scripts/run-demo-worker.sh",
    ROOT / "examples/java/spring-boot4-worker-demo/scripts/run-demo-worker.sh",
]
START_JAVA_DEMOS = ROOT / "scripts/start-java-demo-workers.sh"
CROSS_LANGUAGE_SMOKE = ROOT / "deploy/smoke/cross-language-worker-parity-smoke.sh"


class JavaDemoWorkerScriptContractTest(unittest.TestCase):
    def test_single_demo_scripts_do_not_override_configured_namespace_or_app(self):
        for path in JAVA_DEMO_SCRIPTS:
            with self.subTest(script=path.relative_to(ROOT)):
                script = path.read_text()
                self.assertNotRegex(script, r'WORKER_NAMESPACE="\$\{TIKEO_WORKER_NAMESPACE:-[^}]+\}"')
                self.assertNotRegex(script, r'WORKER_APP="\$\{TIKEO_WORKER_APP:-[^}]+\}"')
                self.assertNotRegex(script, r'(?m)^TIKEO_WORKER_NAMESPACE="\$WORKER_NAMESPACE" \\')
                self.assertNotRegex(script, r'(?m)^TIKEO_WORKER_APP="\$WORKER_APP" \\')

    def test_java_demo_scripts_let_default_election_domain_follow_worker_scope(self):
        for path in JAVA_DEMO_SCRIPTS:
            with self.subTest(script=path.relative_to(ROOT)):
                script = path.read_text()
                self.assertNotIn('$WORKER_NAMESPACE/$WORKER_APP/$WORKER_POOL/$WORKER_CLUSTER', script)
                self.assertNotRegex(script, r'(?m)^TIKEO_WORKER_ELECTION_DOMAIN="\$ELECTION_DOMAIN" \\')

    def test_batch_and_smoke_launchers_do_not_fragment_election_by_worker_pool(self):
        for path in [START_JAVA_DEMOS, CROSS_LANGUAGE_SMOKE]:
            with self.subTest(script=path.relative_to(ROOT)):
                script = path.read_text()
                self.assertNotIn('$namespace/$app/$pool/local', script)
                self.assertNotIn('${namespace}/${app}/${pool}/local', script)


    def test_cross_language_smoke_has_optional_soak_gate_and_evidence_contract(self):
        script = CROSS_LANGUAGE_SMOKE.read_text()
        for token in [
            "TIKEO_CROSS_SOAK_SECONDS",
            "TIKEO_CROSS_SOAK_INTERVAL_SECONDS",
            "SOAK_METRICS_JSONL",
            "SOAK_CSV",
            "SOAK_JSON",
            "run_soak_jobs",
            "cross-language-soak",
            "soak-summary.json",
            "soak-summary.csv",
            "soak-metrics.jsonl",
            "queuePending",
            "outboxPending",
            "workersOnline",
        ]:
            self.assertIn(token, script)
        self.assertRegex(script, r"if \(\( SOAK_SECONDS <= 0 \)\); then")
        self.assertIn("run_language_jobs\n  run_soak_jobs\n  verify_restart_snapshot", script)


if __name__ == "__main__":
    unittest.main()
