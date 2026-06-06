"""Dynamic protobuf loader for bundled Worker Tunnel proto."""

from __future__ import annotations

import importlib
import sys
from functools import lru_cache
from pathlib import Path


@lru_cache(maxsize=1)
def worker_modules():
    try:
        from grpc_tools import protoc
    except ImportError as exc:  # pragma: no cover - dependency guard
        raise RuntimeError("grpcio-tools is required to load tikee worker proto") from exc
    proto = Path(__file__).parent / "proto" / "worker.proto"
    out = Path.home() / ".cache" / "tikee" / "python-proto"
    stamp = out / ".worker.stamp"
    out.mkdir(parents=True, exist_ok=True)
    if not stamp.exists() or stamp.read_text(encoding="utf-8") != proto.read_text(encoding="utf-8"):
        code = protoc.main(["grpc_tools.protoc", f"-I{proto.parent}", f"--python_out={out}", f"--grpc_python_out={out}", str(proto)])
        if code != 0:
            raise RuntimeError(f"failed to generate tikee worker proto: exit={code}")
        stamp.write_text(proto.read_text(encoding="utf-8"), encoding="utf-8")
    if str(out) not in sys.path:
        sys.path.insert(0, str(out))
    return importlib.import_module("worker_pb2"), importlib.import_module("worker_pb2_grpc")
