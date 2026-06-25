#!/usr/bin/env bash
set -euo pipefail

test -f deploy/compose/tikeo.env.example
test -f deploy/systemd/tikeo.service
test -f deploy/systemd/tikeo.env
test -x deploy/bare-metal/check-config.sh
test -f deploy/helm/tikeo/values.yaml
test -f deploy/helm/tikeo/examples/values-external-postgres.yaml
test -f deploy/helm/tikeo/examples/values-ingress-tls.yaml
test -f deploy/helm/tikeo/templates/gateway-api.yaml
test -f deploy/helm/tikeo/templates/servicemonitor.yaml
test -f deploy/helm/tikeo/templates/networkpolicy.yaml
test -f deploy/helm/tikeo/templates/pdb.yaml
test -f deploy/helm/tikeo/examples/values-gateway-api-worker-tunnel.yaml
test -f deploy/helm/tikeo/examples/values-ops-hardening.yaml
test -f deploy/helm/tikeo/examples/values-raft-ha.yaml
test -f deploy/k8s/tikeo-raft-ha.yaml
test -f deploy/helm/tikeo/values.schema.json

grep -q 'TIKEO_CONFIG=/etc/tikeo/tikeo.yml' deploy/systemd/tikeo.env
grep -q 'ExecStart=/opt/tikeo/bin/tikeo serve --config' deploy/systemd/tikeo.service
grep -q 'Service behavior defaults live in ./config/tikeo.yml' deploy/compose/tikeo.env.example
grep -q 'storage.database' config/tikeo.yml
! grep -q '^TIKEO__STORAGE__DATABASE__TYPE=' deploy/compose/tikeo.env.example
grep -q 'mode: external' deploy/helm/tikeo/examples/values-external-postgres.yaml
grep -q 'existingSecret: tikeo-database' deploy/helm/tikeo/examples/values-external-postgres.yaml
grep -q 'mtlsRequired: true' deploy/helm/tikeo/examples/values-ingress-tls.yaml
grep -q 'Rollback' deploy/helm/tikeo/README.md
grep -q 'workers connect outbound' deploy/helm/tikeo/README.md
grep -q 'NetworkPolicy' deploy/helm/tikeo/templates/networkpolicy.yaml
grep -q 'GRPCRoute' deploy/helm/tikeo/templates/gateway-api.yaml
grep -q 'Gateway API' deploy/helm/tikeo/README.md
grep -q 'ServiceMonitor' deploy/helm/tikeo/README.md
grep -q 'NetworkPolicy' deploy/helm/tikeo/README.md
grep -q 'PodDisruptionBudget' deploy/helm/tikeo/README.md
grep -q 'server.cluster.mode' deploy/helm/tikeo/README.md
grep -q 'StatefulSet' deploy/helm/tikeo/README.md
grep -q 'tikeo-server-headless' deploy/k8s/tikeo-raft-ha.yaml
grep -q 'TIKEO__CLUSTER__TRANSPORT_TOKEN' deploy/k8s/tikeo-raft-ha.yaml

echo 'deployment bootstrap templates verified'
