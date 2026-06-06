# tikeo Go Worker SDK

Go SDK for active outbound Worker Tunnel clients and app-scoped management API calls.

This package provides:

- worker configuration validation
- structured Worker capabilities: tags, SDK processors, script runners, and plugin processors
- real Worker Tunnel registration, heartbeat, task result, and graceful unregister helpers
- task processor/outcome interfaces
- `grpc.ClientConn` creation with endpoint normalization
- generated Worker Tunnel client bindings in `internal/workerpb`
- app-scoped management client helpers for SDK, plugin, and script jobs

Dispatch routing must use structured capability fields. Legacy free-form `Capabilities` remain operator metadata only.


## Dynamic script runners

The Go SDK supports structured script runner registration and dynamic script binding execution through `ScriptRunnerRegistry`. A runner is advertised through `WorkerConfig.AddScriptRunner` and must also be registered with the session via `ProcessNextWithScriptRunners`; otherwise script dispatch fails closed instead of pretending to execute.

Go exposes the same structured sandbox backend names as Java: `auto`, `srt`, `deno`, `v8`, `wasmtime`, `wasmedge`, `docker`, `podman`, and `custom`. JavaScript and TypeScript resolve `auto` to `deno`; the other Java-parity demo languages resolve `auto` to `srt`.

`SandboxToolResolver` checks and optionally installs the lightweight sandbox tools used by the default auto path: SRT + ripgrep for native scripts and Deno for JavaScript/TypeScript. Docker/Podman remain explicit heavier backends and are never selected by default. `UnavailableScriptRunner` is useful when code needs a registered fail-closed handler for an unconfigured backend, but it is never advertised as an executable Worker capability. `LocalCommandScriptRunner` is development-only and must advertise `custom`; do not label a worker as `srt`, `deno`, `v8`, `docker`, or `podman` unless that real sandbox boundary is configured and executable.

```go
resolver := tikeo.NewSandboxToolResolver()
scripts := tikeo.NewScriptRunnerRegistry()
if srt, srtOK := resolver.ResolveSrt(); srtOK {
    if rg, rgOK := resolver.ResolveRipgrep(); rgOK {
        runner, err := tikeo.NewSrtScriptRunner("python", srt, "python3", filepath.Dir(rg))
        if err == nil {
            scripts.Register(runner)
        }
    }
}
if deno, ok := resolver.ResolveDeno(); ok {
    runner, err := tikeo.NewDenoScriptRunner("javascript", deno)
    if err == nil {
        scripts.Register(runner)
    }
}
scripts.AddCapabilities(&config)

outcome, err := session.ProcessNextWithScriptRunners(ctx, processor, scripts)
```

## Build and test

The generated Worker Tunnel protobuf bindings are committed under `internal/workerpb`, so an application that only imports and runs the SDK does not need `protoc` at runtime.

`protoc` is required in the Go SDK development environment when you regenerate protobuf bindings with `scripts/generate-workerpb.sh`. CI images and developer containers that run this script must install:

- `protoc`, the Protocol Buffers compiler
- `protoc-gen-go`
- `protoc-gen-go-grpc`

```bash
cd sdks/go/tikeo
go test ./...
```

## Regenerating protobuf bindings

The vendored proto is generated with official `protoc-gen-go` / `protoc-gen-go-grpc`; `scripts/generate-workerpb.sh` regenerates bindings and splits the generated protobuf file into sub-1500-line package files to preserve the repository source-size rule.

```bash
cd sdks/go/tikeo
go install google.golang.org/protobuf/cmd/protoc-gen-go@v1.36.11
go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@v1.6.2
export PATH="$(go env GOPATH)/bin:$PATH"
./scripts/generate-workerpb.sh
```

If `protoc` is missing, the script fails with `protoc is required on PATH`.

## Installing `protoc` locally

Use the system package manager for the base compiler, then install the Go plugins with `go install` as shown above.

```bash
# Debian / Ubuntu
sudo apt-get update
sudo apt-get install -y protobuf-compiler

# Fedora / RHEL / CentOS Stream
sudo dnf install -y protobuf-compiler

# Alpine
sudo apk add --no-cache protobuf

# Arch Linux
sudo pacman -S --needed protobuf

# macOS with Homebrew
brew install protobuf
```

## Dockerfile examples

### Debian / Ubuntu based Go image

```Dockerfile
FROM golang:1.25-bookworm AS build

RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

RUN go install google.golang.org/protobuf/cmd/protoc-gen-go@v1.36.11 \
    && go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@v1.6.2
ENV PATH="/go/bin:${PATH}"

WORKDIR /src
COPY . .
RUN cd sdks/go/tikeo && ./scripts/generate-workerpb.sh && go test ./...
```

### Alpine based Go image

```Dockerfile
FROM golang:1.25-alpine AS build

RUN apk add --no-cache protobuf
RUN go install google.golang.org/protobuf/cmd/protoc-gen-go@v1.36.11 \
    && go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@v1.6.2
ENV PATH="/go/bin:${PATH}"

WORKDIR /src
COPY . .
RUN cd sdks/go/tikeo && ./scripts/generate-workerpb.sh && go test ./...
```

### Fedora based Go image

```Dockerfile
FROM fedora:43 AS build

RUN dnf install -y golang protobuf-compiler \
    && dnf clean all
RUN go install google.golang.org/protobuf/cmd/protoc-gen-go@v1.36.11 \
    && go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@v1.6.2
ENV PATH="/root/go/bin:${PATH}"

WORKDIR /src
COPY . .
RUN cd sdks/go/tikeo && ./scripts/generate-workerpb.sh && go test ./...
```

### Runtime image for applications using committed generated code

If the image only builds or runs an application that imports the SDK and does not call `scripts/generate-workerpb.sh`, it can omit `protoc`:

```Dockerfile
FROM golang:1.25-bookworm AS build
WORKDIR /src
COPY . .
RUN go test ./... && go build -o /out/worker ./examples/go/worker-demo

FROM gcr.io/distroless/base-debian12
COPY --from=build /out/worker /worker
ENTRYPOINT ["/worker"]
```
