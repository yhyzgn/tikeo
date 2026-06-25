#!/usr/bin/env bash
set -euo pipefail

config_path="${1:-config/dev.yml}"
if [[ ! -f "$config_path" ]]; then
  echo "config file not found: $config_path" >&2
  exit 2
fi

cargo run -- serve --config "$config_path" >/tmp/tikeo-check-config.out 2>/tmp/tikeo-check-config.err &
pid=$!
trap 'kill "$pid" >/dev/null 2>&1 || true' EXIT
sleep 2
if ! kill -0 "$pid" >/dev/null 2>&1; then
  cat /tmp/tikeo-check-config.err >&2
  exit 1
fi
echo "tikeo config started successfully: $config_path"
