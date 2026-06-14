from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
DEV_SEED_SQL = ROOT / "scripts" / "dev-seed.sql"
DEV_INTEGRATION_SEED = ROOT / "scripts" / "dev-integration-seed.sh"
START_JAVA_DEMOS = ROOT / "scripts" / "start-java-demo-workers.sh"


EXPECTED_DEMO_POOLS = {
    ("dev-alpha", "orders", "boot2-blue"),
    ("dev-alpha", "orders", "boot3-blue"),
    ("dev-alpha", "orders", "go-blue"),
    ("dev-alpha", "orders", "rust-blue"),
    ("dev-alpha", "orders", "python-blue"),
    ("dev-alpha", "orders", "nodejs-blue"),
    ("dev-alpha", "billing", "boot4-green"),
    ("dev-beta", "analytics", "boot3-batch"),
    ("dev-ops", "automation", "boot4-ops"),
}

EXPECTED_LANGUAGE_DEFAULTS = {
    "go": ("dev-alpha", "orders", "go-blue"),
    "rust": ("dev-alpha", "orders", "rust-blue"),
    "python": ("dev-alpha", "orders", "python-blue"),
    "nodejs": ("dev-alpha", "orders", "nodejs-blue"),
    "java-boot2": ("dev-alpha", "orders", "boot2-blue"),
    "java-boot3": ("dev-alpha", "orders", "boot3-blue"),
    "java-boot4": ("dev-alpha", "billing", "boot4-green"),
}

RUNNABLE_SQL_SEED_JOBS = {
    "job-dev-api-hello",
    "job-dev-fixed-rate-heartbeat",
    "job-dev-cron-minute-report",
    "job-dev-notify-exception",
    "job-dev-notify-success",
}


def text(path: str) -> str:
    return (ROOT / path).read_text()


def env_or_default(source: str, env_name: str) -> str:
    match = re.search(rf'envOr\("{env_name}",\s*"([^"]+)"\)', source)
    if match:
        return match.group(1)
    match = re.search(rf'env_or\("{env_name}",\s*"([^"]+)"\)', source)
    if match:
        return match.group(1)
    raise AssertionError(f"default for {env_name} not found")


def java_property_default(source: str, property_name: str) -> str:
    match = re.search(rf"{re.escape(property_name)}:\s*\$\{{[^:}}]+:([^}}]+)}}", source)
    if not match:
        raise AssertionError(f"default for Java property {property_name} not found")
    return match.group(1)


def sql_apps_by_id(sql: str) -> dict[str, tuple[str, str]]:
    namespaces = {
        row_id: name
        for row_id, name in re.findall(r"\('([^']+)', '([^']+)', '2026-01-01T00:00:00Z'", sql)
        if row_id.startswith("ns-dev-")
    }
    apps = {}
    for app_id, namespace_id, name in re.findall(
        r"\('([^']+)', '(ns-dev-[^']+)', '([^']+)', '2026-01-01T00:00:00Z'",
        sql,
    ):
        if app_id.startswith("app-dev-"):
            apps[app_id] = (namespaces[namespace_id], name)
    return apps


class DemoSeedTopologyContractTest(unittest.TestCase):
    def test_language_demo_defaults_are_part_of_trial_topology(self):
        defaults = {
            "go": (
                env_or_default(text("examples/go/worker-demo/main.go"), "TIKEO_WORKER_NAMESPACE"),
                env_or_default(text("examples/go/worker-demo/main.go"), "TIKEO_WORKER_APP"),
                env_or_default(text("examples/go/worker-demo/main.go"), "TIKEO_WORKER_POOL"),
            ),
            "rust": (
                env_or_default(text("examples/rust/worker-demo/src/main.rs"), "TIKEO_WORKER_NAMESPACE"),
                env_or_default(text("examples/rust/worker-demo/src/main.rs"), "TIKEO_WORKER_APP"),
                env_or_default(text("examples/rust/worker-demo/src/main.rs"), "TIKEO_WORKER_POOL"),
            ),
            "python": (
                env_or_default(text("examples/python/worker-demo/src/tikeo_python_worker_demo/__main__.py"), "TIKEO_WORKER_NAMESPACE"),
                env_or_default(text("examples/python/worker-demo/src/tikeo_python_worker_demo/__main__.py"), "TIKEO_WORKER_APP"),
                env_or_default(text("examples/python/worker-demo/src/tikeo_python_worker_demo/__main__.py"), "TIKEO_WORKER_POOL"),
            ),
            "nodejs": (
                env_or_default(text("examples/nodejs/worker-demo/src/main.ts"), "TIKEO_WORKER_NAMESPACE"),
                env_or_default(text("examples/nodejs/worker-demo/src/main.ts"), "TIKEO_WORKER_APP"),
                env_or_default(text("examples/nodejs/worker-demo/src/main.ts"), "TIKEO_WORKER_POOL"),
            ),
            "java-boot2": (
                java_property_default(text("examples/java/spring-boot2-worker-demo/src/main/resources/application.yml"), "namespace"),
                java_property_default(text("examples/java/spring-boot2-worker-demo/src/main/resources/application.yml"), "app"),
                java_property_default(text("examples/java/spring-boot2-worker-demo/src/main/resources/application.yml"), "worker_pool"),
            ),
            "java-boot3": (
                java_property_default(text("examples/java/spring-boot3-worker-demo/src/main/resources/application.yml"), "namespace"),
                java_property_default(text("examples/java/spring-boot3-worker-demo/src/main/resources/application.yml"), "app"),
                java_property_default(text("examples/java/spring-boot3-worker-demo/src/main/resources/application.yml"), "worker_pool"),
            ),
            "java-boot4": (
                java_property_default(text("examples/java/spring-boot4-worker-demo/src/main/resources/application.yml"), "namespace"),
                java_property_default(text("examples/java/spring-boot4-worker-demo/src/main/resources/application.yml"), "app"),
                java_property_default(text("examples/java/spring-boot4-worker-demo/src/main/resources/application.yml"), "worker_pool"),
            ),
        }
        self.assertEqual(defaults, EXPECTED_LANGUAGE_DEFAULTS)
        self.assertTrue(set(defaults.values()).issubset(EXPECTED_DEMO_POOLS))

    def test_sql_seed_contains_every_demo_worker_pool(self):
        sql = DEV_SEED_SQL.read_text()
        apps = sql_apps_by_id(sql)
        pools = set()
        for namespace_id, app_id, pool_name in re.findall(
            r"\('wp-dev-[^']+', '(ns-dev-[^']+)', '(app-dev-[^']+)', '([^']+)'",
            sql,
        ):
            namespace, app = apps[app_id]
            pools.add((namespace, app, pool_name))

        self.assertTrue(EXPECTED_DEMO_POOLS.issubset(pools), pools)
        self.assertEqual(len([pool for pool in pools if pool in EXPECTED_DEMO_POOLS]), len(EXPECTED_DEMO_POOLS))

    def test_api_seed_contains_every_demo_worker_pool(self):
        script = DEV_INTEGRATION_SEED.read_text()
        pools = {
            (namespace, app, pool)
            for namespace, app, pool in re.findall(
                r"^create_pool\s+(\S+)\s+(\S+)\s+(\S+)\s+\d+\s+\d+",
                script,
                re.MULTILINE,
            )
        }
        self.assertEqual(pools, EXPECTED_DEMO_POOLS)

    def test_java_matrix_uses_seeded_worker_pools(self):
        launcher = START_JAVA_DEMOS.read_text()
        workers = {
            (namespace, app, pool)
            for namespace, app, pool in re.findall(
                r'"[^"]+\|examples/java/[^|]+\|\d+\|([^|]+)\|([^|]+)\|([^|]+)\|\d+"',
                launcher,
            )
        }
        self.assertTrue(workers.issubset(EXPECTED_DEMO_POOLS))
        self.assertIn(("dev-alpha", "orders", "boot2-blue"), workers)
        self.assertIn(("dev-alpha", "orders", "boot3-blue"), workers)
        self.assertIn(("dev-alpha", "billing", "boot4-green"), workers)


    def test_notification_progress_demo_instance_is_terminal_not_permanently_running(self):
        sql = DEV_SEED_SQL.read_text()
        instance_match = re.search(
            r"\('inst-dev-notify-feishu-running', 'job-dev-notify-success', '([^']+)', 'api', 'single', '[^']+', ([^,]+), '([^']+)', '([^']+)'",
            sql,
        )
        self.assertIsNotNone(instance_match)
        self.assertEqual(instance_match.group(1), "succeeded")
        self.assertEqual(instance_match.group(2), "1")
        self.assertIn("after progress notification", instance_match.group(3))
        self.assertNotEqual(instance_match.group(4), "NULL")

        attempt_match = re.search(
            r"\('attempt-dev-notify-feishu-running-1', 'inst-dev-notify-feishu-running', '[^']+', '([^']+)', ([^,]+), '([^']+)', '([^']+)'",
            sql,
        )
        self.assertIsNotNone(attempt_match)
        self.assertEqual(attempt_match.group(1), "succeeded")
        self.assertEqual(attempt_match.group(2), "1")
        self.assertIn("after progress notification", attempt_match.group(3))
        self.assertNotEqual(attempt_match.group(4), "NULL")

        self.assertIn("'job_instance.running'", sql)
        self.assertIn('"eventType":"job_instance.running"', sql)
        self.assertIn('"finishedAt":null', sql)

    def test_direct_sql_runnable_jobs_share_default_demo_scope(self):
        sql = DEV_SEED_SQL.read_text()
        apps = sql_apps_by_id(sql)
        job_scopes = {}
        for job_id, namespace_id, app_id in re.findall(
            r"\('([^']+)', '(ns-dev-[^']+)', '(app-dev-[^']+)'",
            sql,
        ):
            if job_id.startswith("job-dev-"):
                job_scopes[job_id] = (namespace_id, apps[app_id])

        for job_id in RUNNABLE_SQL_SEED_JOBS:
            with self.subTest(job=job_id):
                self.assertIn(job_id, job_scopes)
                self.assertEqual(job_scopes[job_id][1], ("dev-alpha", "orders"))


if __name__ == "__main__":
    unittest.main()
