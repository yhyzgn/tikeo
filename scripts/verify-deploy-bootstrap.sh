#!/usr/bin/env bash
set -euo pipefail

test -f deploy/compose/tikeo.env.example
test -f deploy/systemd/tikeo.service
test -f deploy/systemd/tikeo.env
test -x deploy/bare-metal/check-config.sh
grep -q 'TIKEO_CONFIG=/etc/tikeo/tikeo.toml' deploy/systemd/tikeo.env
grep -q 'ExecStart=/opt/tikeo/bin/tikeo serve --config' deploy/systemd/tikeo.service
grep -q 'TIKEO__STORAGE__DATABASE_URL' deploy/compose/tikeo.env.example
grep -q 'Helm remains deferred' deploy/README.md

echo 'deployment bootstrap templates verified'
