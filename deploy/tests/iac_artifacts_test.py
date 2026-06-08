from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (ROOT / path).read_text()


class IacArtifactsTest(unittest.TestCase):
    def test_terraform_provider_has_real_resource_and_data_source_contracts(self):
        provider = read('deploy/terraform/provider/provider.go')
        resource = read('deploy/terraform/provider/internal/provider/manifest_diff_resource.go')
        data_source = read('deploy/terraform/provider/internal/provider/manifest_data_source.go')
        self.assertIn('github.com/hashicorp/terraform-plugin-framework/provider', provider)
        self.assertIn('tikeo_manifest_diff', resource)
        self.assertIn('/api/v1/gitops/diff', resource)
        self.assertIn('tikeo_manifest', data_source)
        self.assertIn('/api/v1/gitops/manifest', data_source)
        self.assertIn('Authorization', read('deploy/terraform/provider/internal/tikeo/client.go'))

    def test_k8s_operator_has_crd_status_and_diff_reconcile_contracts(self):
        crd = read('deploy/k8s/crd/tikeo-manifest-crd.yaml')
        self.assertIn('status:', crd)
        self.assertIn('observedGeneration', crd)
        self.assertIn('checksum', crd)
        self.assertIn('conditions', crd)
        controller = read('deploy/k8s/operator/internal/controller/reconciler.go')
        self.assertIn('/api/v1/gitops/diff', controller)
        self.assertIn('status', controller.lower())
        self.assertIn('applyMode', controller)
        self.assertIn('diffOnly', controller)
        main = read('deploy/k8s/operator/cmd/tikeo-operator/main.go')
        self.assertIn('tikeo-operator', main)
        self.assertIn('kubeconfig', main)

    def test_helm_chart_exposes_production_hardening_contracts(self):
        values = read('deploy/helm/tikeo/values.yaml')
        server = read('deploy/helm/tikeo/templates/server.yaml')
        configmap = read('deploy/helm/tikeo/templates/configmap.yaml')
        web = read('deploy/helm/tikeo/templates/web.yaml')
        readme = read('deploy/helm/tikeo/README.md')

        # External database credentials must come from Kubernetes Secrets, not inline values.
        self.assertIn('mode: sqlite', values)
        self.assertIn('existingSecret:', values)
        self.assertIn('databaseUrlSecretKey: database-url', values)
        self.assertIn('TIKEO__STORAGE__DATABASE_URL', server)
        self.assertIn('secretKeyRef', server)
        self.assertIn('eq .Values.server.storage.mode "sqlite"', server)

        # Real listener TLS/mTLS settings must be wired into the generated config and mounted secrets.
        self.assertIn('[transport_security.http]', configmap)
        self.assertIn('[transport_security.worker_tunnel]', configmap)
        self.assertIn('tls.crt', server)
        self.assertIn('tls.key', server)
        self.assertIn('ca.crt', server)

        # Production workloads should expose tunable probes, resources, contexts and ingress.
        self.assertIn('serviceAccount', values)
        self.assertIn('podSecurityContext', values)
        self.assertIn('securityContext', values)
        self.assertIn('resources:', server)
        self.assertIn('readinessProbe', web)
        self.assertIn('kind: Ingress', web)

        # Docs/examples must cover external DB, TLS/mTLS, worker identity and rollback operations.
        self.assertIn('values-external-postgres.yaml', readme)
        self.assertIn('values-ingress-tls.yaml', readme)
        self.assertIn('Rollback', readme)
        self.assertIn('worker identity', readme.lower())

    def test_helm_chart_exposes_operational_maturity_contracts(self):
        values = read('deploy/helm/tikeo/values.yaml')
        schema = read('deploy/helm/tikeo/values.schema.json')
        pdb = read('deploy/helm/tikeo/templates/pdb.yaml')
        network_policy = read('deploy/helm/tikeo/templates/networkpolicy.yaml')
        service_monitor = read('deploy/helm/tikeo/templates/servicemonitor.yaml')
        gateway = read('deploy/helm/tikeo/templates/gateway-api.yaml')
        readme = read('deploy/helm/tikeo/README.md')

        self.assertIn('pdb:', values)
        self.assertIn('networkPolicy:', values)
        self.assertIn('serviceMonitor:', values)
        self.assertIn('gatewayApi:', values)

        self.assertIn('PodDisruptionBudget', pdb)
        self.assertIn('minAvailable', pdb)
        self.assertIn('NetworkPolicy', network_policy)
        self.assertIn('workers-connect-outbound-only', network_policy)
        self.assertIn('ServiceMonitor', service_monitor)
        self.assertIn('/metrics', service_monitor)
        self.assertIn('GRPCRoute', gateway)
        self.assertIn('worker-tunnel', gateway)

        self.assertIn('"values.schema.json"', schema)
        self.assertIn('"enum": ["sqlite", "external"]', schema)
        self.assertIn('"gatewayApi"', schema)

        self.assertIn('PodDisruptionBudget', readme)
        self.assertIn('NetworkPolicy', readme)
        self.assertIn('ServiceMonitor', readme)
        self.assertIn('Gateway API', readme)

    def test_roadmap_and_coverage_mark_iac_closed(self):
        design = read('design/tikeo-architecture-design.md')
        coverage = read('design/reports/feature-coverage-competitive-checklist.md')
        self.assertIn('Terraform Provider 与 K8s CRD controller/operator 已补齐', design)
        self.assertIn('| GitOps/IaC | ✅ 已覆盖 |', coverage)
        self.assertNotIn('Terraform Provider/K8s CRD 控制器仍为 P2 缺口', coverage)


if __name__ == '__main__':
    unittest.main()
