#!/usr/bin/env bash
set -euo pipefail

test -f deploy/compose/tikee.env.example
test -f deploy/systemd/tikee.service
test -f deploy/systemd/tikee.env
test -x deploy/bare-metal/check-config.sh
grep -q 'TIKEE_CONFIG=/etc/tikee/tikee.toml' deploy/systemd/tikee.env
grep -q 'ExecStart=/opt/tikee/bin/tikee serve --config' deploy/systemd/tikee.service
grep -q 'TIKEE__STORAGE__DATABASE_URL' deploy/compose/tikee.env.example
grep -q 'Helm remains deferred' deploy/README.md

echo 'deployment bootstrap templates verified'
