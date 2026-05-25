#!/usr/bin/env bash
set -euo pipefail

base_url="${TIKEE_HTTP_URL:-http://127.0.0.1:9090}"
endpoint="${TIKEE_WORKER_ENDPOINT:-http://127.0.0.1:9998}"

curl -fsS "$base_url/readyz" >/tmp/tikee-readyz.json
printf 'readyz ok: %s\n' "$base_url/readyz"

if [[ "${TIKEE_SMOKE_RUN_RUST_WORKER:-1}" == "1" ]]; then
  cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
else
  printf 'worker dry-run skipped; set TIKEE_SMOKE_RUN_RUST_WORKER=1 to run the Rust demo config check.\n'
fi

printf 'worker endpoint configured: %s\n' "$endpoint"
printf 'smoke complete without opening inbound business ports.\n'
