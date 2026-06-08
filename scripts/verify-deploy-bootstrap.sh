#!/usr/bin/env bash
set -euo pipefail

test -f deploy/compose/tikeo.env.example
test -f deploy/systemd/tikeo.service
test -f deploy/systemd/tikeo.env
test -x deploy/bare-metal/check-config.sh
test -f deploy/helm/tikeo/values.yaml
test -f deploy/helm/tikeo/examples/values-external-postgres.yaml
test -f deploy/helm/tikeo/examples/values-ingress-tls.yaml

grep -q 'TIKEO_CONFIG=/etc/tikeo/tikeo.toml' deploy/systemd/tikeo.env
grep -q 'ExecStart=/opt/tikeo/bin/tikeo serve --config' deploy/systemd/tikeo.service
grep -q 'TIKEO__STORAGE__DATABASE_URL' deploy/compose/tikeo.env.example
grep -q 'mode: external' deploy/helm/tikeo/examples/values-external-postgres.yaml
grep -q 'existingSecret: tikeo-database' deploy/helm/tikeo/examples/values-external-postgres.yaml
grep -q 'mtlsRequired: true' deploy/helm/tikeo/examples/values-ingress-tls.yaml
grep -q 'Rollback' deploy/helm/tikeo/README.md
grep -q 'workers connect outbound' deploy/helm/tikeo/README.md

echo 'deployment bootstrap templates verified'
