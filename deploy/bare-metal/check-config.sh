#!/usr/bin/env bash
set -euo pipefail

config_path="${1:-config/dev.toml}"
if [[ ! -f "$config_path" ]]; then
  echo "config file not found: $config_path" >&2
  exit 2
fi

cargo run -- serve --config "$config_path" >/tmp/tikee-check-config.out 2>/tmp/tikee-check-config.err &
pid=$!
trap 'kill "$pid" >/dev/null 2>&1 || true' EXIT
sleep 2
if ! kill -0 "$pid" >/dev/null 2>&1; then
  cat /tmp/tikee-check-config.err >&2
  exit 1
fi
echo "tikee config started successfully: $config_path"
