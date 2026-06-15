import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HELM = ROOT / ".dev/tools/helm"
CHART = ROOT / "deploy/helm/tikeo"
RAFT_VALUES = CHART / "examples/values-raft-ha.yaml"


def helm_template(*extra_args: str) -> str:
    result = subprocess.run(
        [str(HELM), "template", "tikeo", str(CHART), "--namespace", "tikeo", *extra_args],
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout


class HelmRaftHaContractTest(unittest.TestCase):
    def test_default_server_remains_standalone_deployment(self):
        rendered = helm_template()

        self.assertIn("kind: Deployment", rendered)
        self.assertIn("name: tikeo-server", rendered)
        self.assertNotIn("name: tikeo-server-headless", rendered)
        self.assertNotIn("TIKEO__CLUSTER__MODE", rendered)
        self.assertNotIn("kind: StatefulSet", rendered)
        self.assertNotIn("          env:\n          ports:", rendered)

    def test_raft_ha_renders_statefulset_headless_service_and_secret_backed_token(self):
        rendered = helm_template("-f", str(RAFT_VALUES))

        self.assertIn("kind: StatefulSet", rendered)
        self.assertIn("name: tikeo-server", rendered)
        self.assertIn("serviceName: tikeo-server-headless", rendered)
        self.assertIn("replicas: 3", rendered)
        self.assertIn("name: tikeo-server-headless", rendered)
        self.assertIn("clusterIP: None", rendered)
        self.assertIn("publishNotReadyAddresses: true", rendered)
        self.assertIn("TIKEO__CLUSTER__MODE", rendered)
        self.assertIn("value: \"raft\"", rendered)
        self.assertIn("TIKEO__CLUSTER__NODE_ID", rendered)
        self.assertIn("fieldPath: metadata.name", rendered)
        self.assertIn("TIKEO__CLUSTER__TRANSPORT_TOKEN", rendered)
        self.assertIn("secretKeyRef:", rendered)
        self.assertIn("name: \"tikeo-raft-transport\"", rendered)
        self.assertIn("key: \"transport-token\"", rendered)
        self.assertIn("endpoint = \"http://tikeo-server-0.tikeo-server-headless:9090\"", rendered)
        self.assertIn("endpoint = \"http://tikeo-server-1.tikeo-server-headless:9090\"", rendered)
        self.assertIn("endpoint = \"http://tikeo-server-2.tikeo-server-headless:9090\"", rendered)

    def test_raw_k8s_raft_manifest_has_stateful_identity_and_secret_refs(self):
        manifest = (ROOT / "deploy/k8s/tikeo-raft-ha.yaml").read_text()

        self.assertIn("kind: StatefulSet", manifest)
        self.assertIn("name: tikeo-server-headless", manifest)
        self.assertIn("clusterIP: None", manifest)
        self.assertIn("publishNotReadyAddresses: true", manifest)
        self.assertIn("replicas: 3", manifest)
        self.assertIn("fieldPath: metadata.name", manifest)
        self.assertIn("TIKEO__CLUSTER__MODE", manifest)
        self.assertIn("TIKEO__CLUSTER__TRANSPORT_TOKEN", manifest)
        self.assertIn("name: tikeo-raft-transport", manifest)
        self.assertIn("key: transport-token", manifest)
        self.assertIn("http://tikeo-server-0.tikeo-server-headless:9090", manifest)


if __name__ == "__main__":
    unittest.main()
