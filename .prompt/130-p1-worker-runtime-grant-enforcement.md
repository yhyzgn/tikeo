# 130 - P1 Worker runtime grant enforcement

## Goal
Close the release-to-runtime boundary for signed script URL/File/Secret grants, including Java SDK protocol awareness.

## Scope
- Extend Worker proto `ScriptProcessorBinding` with `allowed_network_hosts` in server, Rust SDK, and Java SDK proto copies.
- Server dispatcher maps verified `release_grants` into runtime bindings and allows non-default-deny script policy only when verified grant evidence exists.
- Rust SDK maps runtime grants into `ScriptRunnerPolicy`.
- Local subprocess runner remains fail-closed for URL/File/Secret grants.
- Container runner supports signed read/write file grants as explicit bind mounts, keeps Docker networking disabled, and fails closed for network/secret grants until a safe host-filtering network sandbox and worker-local secret provider exist.
- Java SDK remains non-executing for script bindings but generated proto/tests cover grant-bearing bindings and unsupported behavior.

## Validation target
- Server dispatch test proves verified release grants are copied into `ScriptProcessorBinding`.
- Rust SDK tests prove container file mount args and fail-closed network/secret/malformed file behavior.
- Java SDK test proves grant-bearing script binding is parsed and still does not invoke the normal processor.
