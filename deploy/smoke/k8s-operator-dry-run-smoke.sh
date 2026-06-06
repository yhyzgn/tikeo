#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR/deploy/k8s/operator"
go test ./...
cd "$ROOT_DIR"
python3 - <<'PY'
from pathlib import Path
crd=Path('deploy/k8s/crd/tikeo-manifest-crd.yaml')
text=crd.read_text(encoding='utf-8')
assert 'kind: CustomResourceDefinition' in text
assert 'TikeoManifest' in text
assert 'openAPIV3Schema' in text
print('k8s CRD dry-run static expectation passed')
PY
