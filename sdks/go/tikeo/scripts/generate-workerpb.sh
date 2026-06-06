#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROTO="$ROOT/proto/worker.proto"
OUT_ROOT="$ROOT"
PKG_DIR="$ROOT/internal/workerpb"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR" "$OUT_ROOT/github.com"' EXIT

command -v protoc >/dev/null || {
  echo "protoc is required on PATH" >&2
  exit 1
}
command -v protoc-gen-go >/dev/null || {
  echo "protoc-gen-go is required on PATH; run: go install google.golang.org/protobuf/cmd/protoc-gen-go@v1.36.11" >&2
  exit 1
}
command -v protoc-gen-go-grpc >/dev/null || {
  echo "protoc-gen-go-grpc is required on PATH; run: go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@v1.6.2" >&2
  exit 1
}

rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR"
protoc -I "$ROOT/proto" \
  --go_out="$OUT_ROOT" \
  --go-grpc_out="$OUT_ROOT" \
  "$PROTO"

generated="$OUT_ROOT/github.com/yhyzgn/tikeo/sdks/go/tikeo/internal/workerpb"
mv "$generated"/* "$PKG_DIR"/
rm -rf "$OUT_ROOT/github.com"

python3 - "$PKG_DIR/worker.pb.go" <<'PY'
from pathlib import Path
import sys
src = Path(sys.argv[1])
text = src.read_text()
marker = 'type ScriptProcessorBinding struct {'
header_end = text.index('\n)\n', text.index('import (')) + len('\n)\n')
idx = text.index(marker)
header = text[:header_end]
body = text[header_end:]
rel_idx = idx - header_end
msg_header = header.replace('\treflect "reflect"\n', '').replace('\tsync "sync"\n', '').replace('\tunsafe "unsafe"\n', '')
(src.parent / 'worker_messages.pb.go').write_text(msg_header + body[:rel_idx].rstrip() + '\n')
(src.parent / 'worker_descriptor.pb.go').write_text(header + body[rel_idx:].rstrip() + '\n')
src.unlink()
PY

gofmt -w "$PKG_DIR"/*.go
