---
title: Worker sandbox tools and Dockerfiles
description: Preinstall Tikeo Worker sandbox tools on hosts or Docker images for default PATH reuse and strict sandbox isolation.
keywords: [tikeo worker, sandbox tools, dockerfile, powershell, deno, srt, wasmtime]
---

# Worker sandbox tools and Dockerfiles

Tikeo Workers can execute SDK processors and optional script/plugin runners. Script runners may need local tools such as SRT, Deno, ripgrep, Rhai, PowerShell, Wasmtime, or WasmEdge.

Production Workers should **not** depend on startup-time downloads. Build or prepare the Worker host/image with the tools you intend to advertise, then disable or de-emphasize SDK auto-install.

## Runtime modes

| Mode | Configuration | Tool lookup behavior | Recommended use |
| --- | --- | --- | --- |
| Default host reuse | `TIKEO_SANDBOX_STRICT_ISOLATION=0` or unset | SDKs first use a working binary from `PATH`, then legacy `state-dir/sandbox-tools`, then `TIKEO_SANDBOX_TOOLS_DIR` / `~/.tikeo/sandbox-tools`. Each task still receives sandbox `cwd`, `HOME`, `TMPDIR`, `DENO_DIR`, and PowerShell/.NET cache directories. | Most production Worker images where the image itself is trusted. |
| Strict sandbox isolation | `TIKEO_SANDBOX_STRICT_ISOLATION=1`; Java Boot also supports `tikeo.worker.scripts.strict-sandbox-isolation=true` | SDKs skip host `PATH` tools/interpreters and only use binaries under `TIKEO_SANDBOX_TOOLS_DIR` or `~/.tikeo/sandbox-tools`. Missing tools are not advertised and fail closed. | Regulated, multi-tenant, or change-controlled Workers. |
| Local/demo auto-prewarm | `TIKEO_SANDBOX_AUTO_INSTALL=1` / SDK defaults | Missing tools schedule background prewarm. Startup does not wait for the download. | Developer laptops and short-lived demos only. |

For production images, set:

```bash
TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools
TIKEO_SANDBOX_AUTO_INSTALL=0
# Optional stronger boundary:
TIKEO_SANDBOX_STRICT_ISOLATION=1
```

## Tool source map

| Tool / binary | Used for | Central package manager support | Manual/upstream install path |
| --- | --- | --- | --- |
| `sh`, `bash` | Shell-backed runners and installers | `apt`, `dnf`, `apk` | Usually built in; strict mode can symlink `/bin/sh` into the cache. |
| `node`, `npm` | SRT launcher and npm-installed tools | Most distro repos, NodeSource, or official Node images | Official Node binary tarball if distro version is too old. |
| `srt` | Anthropic Sandbox Runtime-backed shell/python/node/powershell execution | npm registry | `npm install -g --prefix /opt/tikeo/sandbox-tools/srt @anthropic-ai/sandbox-runtime`. |
| `rg` | ripgrep dependency used by SRT | Debian/Ubuntu/Fedora/RHEL/Alpine packages; crates.io | `cargo install --root /opt/tikeo/sandbox-tools/rg ripgrep`. Java also accepts `/opt/tikeo/sandbox-tools/ripgrep/bin/rg`. |
| `deno` | JavaScript/TypeScript sandbox execution | Not consistently packaged in base distro repos | Official installer or GitHub release zip into `/opt/tikeo/sandbox-tools/deno/bin/deno`. |
| `rhai-run` | Rhai script execution | crates.io | `cargo install --root /opt/tikeo/sandbox-tools/rhai-run rhai --bins --features bin-features`. Java also accepts `/opt/tikeo/sandbox-tools/rhai/bin/rhai-run`. |
| `pwsh` | PowerShell scripts | Microsoft package repos for supported Debian/Ubuntu/RHEL-family images; Alpine normally uses tarball | Download `powershell-${version}-linux-x64.tar.gz` or `linux-arm64.tar.gz` from PowerShell GitHub Releases and extract under `/opt/tikeo/sandbox-tools/pwsh`. |
| `wasmtime` | WASM runtime execution | Usually upstream installer or release archive; cargo fallback possible | `curl https://wasmtime.dev/install.sh -sSf | bash`, then copy/symlink `wasmtime` into the cache. |
| `wasmedge` | Optional WasmEdge backend | Fedora/EPEL packages where available; otherwise upstream script | `curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash`, then copy/symlink `wasmedge` into the cache. |

:::tip Mixed SDK images
Create both compatibility cache names where current SDKs differ: `rg` and `ripgrep` for ripgrep, `rhai-run` and `rhai` for Rhai. This lets Java and Go/Python/Node/Rust Workers share the same base image.
:::

## Official install references

Use these upstream pages when pinning versions or adapting the Dockerfiles to an internal mirror:

- PowerShell: [Linux support overview](https://learn.microsoft.com/en-us/powershell/scripting/install/linux-overview) and [tar.gz archive install](https://learn.microsoft.com/en-us/powershell/scripting/install/alternate-install-methods).
- Deno: [Installation](https://docs.deno.com/runtime/getting_started/installation/).
- Wasmtime: [CLI installation](https://docs.wasmtime.dev/cli-install.html).
- WasmEdge: [Install and uninstall WasmEdge](https://wasmedge.org/docs/start/install/).

## Cache layout expected by strict mode

```text
/opt/tikeo/sandbox-tools/
  sh/bin/sh
  node/bin/node
  npm/bin/npm
  srt/bin/srt
  rg/bin/rg
  ripgrep/bin/rg
  deno/bin/deno
  rhai-run/bin/rhai-run
  rhai/bin/rhai-run
  pwsh/bin/pwsh
  wasmtime/bin/wasmtime
  wasmedge/bin/wasmedge
```

Minimum Dockerfile bootstrap used by all examples:

```docker
ENV TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools \
    TIKEO_SANDBOX_STRICT_ISOLATION=1 \
    TIKEO_SANDBOX_AUTO_INSTALL=0

RUN mkdir -p \
      ${TIKEO_SANDBOX_TOOLS_DIR}/sh/bin \
      ${TIKEO_SANDBOX_TOOLS_DIR}/node/bin \
      ${TIKEO_SANDBOX_TOOLS_DIR}/npm/bin \
      ${TIKEO_SANDBOX_TOOLS_DIR}/srt/bin \
      ${TIKEO_SANDBOX_TOOLS_DIR}/rg/bin \
      ${TIKEO_SANDBOX_TOOLS_DIR}/ripgrep/bin \
      ${TIKEO_SANDBOX_TOOLS_DIR}/rhai-run/bin \
      ${TIKEO_SANDBOX_TOOLS_DIR}/rhai/bin \
      ${TIKEO_SANDBOX_TOOLS_DIR}/deno/bin \
      ${TIKEO_SANDBOX_TOOLS_DIR}/pwsh/bin \
      ${TIKEO_SANDBOX_TOOLS_DIR}/wasmtime/bin \
      ${TIKEO_SANDBOX_TOOLS_DIR}/wasmedge/bin
```

## Debian / Ubuntu Worker Dockerfile

Use this for Debian, Ubuntu, and common Java/Python/Node base images.

```docker
FROM eclipse-temurin:21-jre-jammy

ARG POWERSHELL_VERSION=7.5.4
ENV TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools \
    TIKEO_SANDBOX_STRICT_ISOLATION=1 \
    TIKEO_SANDBOX_AUTO_INSTALL=0

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates curl tar gzip unzip xz-utils bash nodejs npm cargo ripgrep \
 && rm -rf /var/lib/apt/lists/*

RUN set -eux; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR"; \
    npm install -g --prefix "$TIKEO_SANDBOX_TOOLS_DIR/srt" @anthropic-ai/sandbox-runtime; \
    cargo install --root "$TIKEO_SANDBOX_TOOLS_DIR/rhai-run" rhai --bins --features bin-features; \
    arch="$(dpkg --print-architecture)"; \
    case "$arch" in amd64) deno_arch=x86_64-unknown-linux-gnu; ps_arch=linux-x64 ;; arm64) deno_arch=aarch64-unknown-linux-gnu; ps_arch=linux-arm64 ;; *) echo "unsupported arch: $arch"; exit 1 ;; esac; \
    curl -fsSL "https://github.com/denoland/deno/releases/latest/download/deno-${deno_arch}.zip" -o /tmp/deno.zip; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin"; \
    unzip -q /tmp/deno.zip -d "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin"; \
    chmod +x "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin/deno"; \
    curl -fsSL "https://github.com/PowerShell/PowerShell/releases/download/v${POWERSHELL_VERSION}/powershell-${POWERSHELL_VERSION}-${ps_arch}.tar.gz" -o /tmp/pwsh.tar.gz; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}" "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/bin"; \
    tar -xzf /tmp/pwsh.tar.gz -C "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}"; \
    chmod +x "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}/pwsh"; \
    ln -sf "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}/pwsh" "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/bin/pwsh"; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/sh/bin" "$TIKEO_SANDBOX_TOOLS_DIR/node/bin" "$TIKEO_SANDBOX_TOOLS_DIR/npm/bin" "$TIKEO_SANDBOX_TOOLS_DIR/rg/bin" "$TIKEO_SANDBOX_TOOLS_DIR/ripgrep/bin" "$TIKEO_SANDBOX_TOOLS_DIR/rhai/bin"; \
    ln -sf /bin/sh "$TIKEO_SANDBOX_TOOLS_DIR/sh/bin/sh"; \
    ln -sf "$(command -v node)" "$TIKEO_SANDBOX_TOOLS_DIR/node/bin/node"; \
    ln -sf "$(command -v npm)" "$TIKEO_SANDBOX_TOOLS_DIR/npm/bin/npm"; \
    ln -sf "$(command -v rg)" "$TIKEO_SANDBOX_TOOLS_DIR/rg/bin/rg"; \
    ln -sf "$(command -v rg)" "$TIKEO_SANDBOX_TOOLS_DIR/ripgrep/bin/rg"; \
    ln -sf "$TIKEO_SANDBOX_TOOLS_DIR/rhai-run/bin/rhai-run" "$TIKEO_SANDBOX_TOOLS_DIR/rhai/bin/rhai-run"; \
    curl https://wasmtime.dev/install.sh -sSf | bash; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/wasmtime/bin"; \
    cp "$HOME/.wasmtime/bin/wasmtime" "$TIKEO_SANDBOX_TOOLS_DIR/wasmtime/bin/wasmtime"; \
    rm -rf /tmp/deno.zip /tmp/pwsh.tar.gz "$HOME/.cargo/registry" "$HOME/.cargo/git"

WORKDIR /app
COPY target/app.jar /app/app.jar
ENTRYPOINT ["java", "-jar", "/app/app.jar"]
```

## RHEL / UBI / Fedora Worker Dockerfile

This example uses Fedora because the required build-time packages are available from the default repositories. For UBI/RHEL minimal images, either enable the required Red Hat repositories for `cargo`/`ripgrep` or use the distroless-style builder pattern below and copy the finished cache into the runtime image.

```docker
FROM fedora:42

ARG POWERSHELL_VERSION=7.5.4
ENV TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools \
    TIKEO_SANDBOX_STRICT_ISOLATION=1 \
    TIKEO_SANDBOX_AUTO_INSTALL=0
RUN dnf install -y ca-certificates curl tar gzip unzip xz bash nodejs npm cargo ripgrep java-21-openjdk-headless \
 && dnf clean all

RUN set -eux; \
    npm install -g --prefix "$TIKEO_SANDBOX_TOOLS_DIR/srt" @anthropic-ai/sandbox-runtime; \
    cargo install --root "$TIKEO_SANDBOX_TOOLS_DIR/rhai-run" rhai --bins --features bin-features; \
    arch="$(uname -m)"; \
    case "$arch" in x86_64) deno_arch=x86_64-unknown-linux-gnu; ps_arch=linux-x64 ;; aarch64) deno_arch=aarch64-unknown-linux-gnu; ps_arch=linux-arm64 ;; *) echo "unsupported arch: $arch"; exit 1 ;; esac; \
    curl -fsSL "https://github.com/denoland/deno/releases/latest/download/deno-${deno_arch}.zip" -o /tmp/deno.zip; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin"; unzip -q /tmp/deno.zip -d "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin"; chmod +x "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin/deno"; \
    curl -fsSL "https://github.com/PowerShell/PowerShell/releases/download/v${POWERSHELL_VERSION}/powershell-${POWERSHELL_VERSION}-${ps_arch}.tar.gz" -o /tmp/pwsh.tar.gz; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}" "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/bin"; \
    tar -xzf /tmp/pwsh.tar.gz -C "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}"; \
    chmod +x "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}/pwsh"; \
    ln -sf "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}/pwsh" "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/bin/pwsh"; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/sh/bin" "$TIKEO_SANDBOX_TOOLS_DIR/node/bin" "$TIKEO_SANDBOX_TOOLS_DIR/npm/bin" "$TIKEO_SANDBOX_TOOLS_DIR/rg/bin" "$TIKEO_SANDBOX_TOOLS_DIR/ripgrep/bin" "$TIKEO_SANDBOX_TOOLS_DIR/rhai/bin" "$TIKEO_SANDBOX_TOOLS_DIR/wasmtime/bin"; \
    ln -sf /bin/sh "$TIKEO_SANDBOX_TOOLS_DIR/sh/bin/sh"; \
    ln -sf "$(command -v node)" "$TIKEO_SANDBOX_TOOLS_DIR/node/bin/node"; \
    ln -sf "$(command -v npm)" "$TIKEO_SANDBOX_TOOLS_DIR/npm/bin/npm"; \
    ln -sf "$(command -v rg)" "$TIKEO_SANDBOX_TOOLS_DIR/rg/bin/rg"; \
    ln -sf "$(command -v rg)" "$TIKEO_SANDBOX_TOOLS_DIR/ripgrep/bin/rg"; \
    ln -sf "$TIKEO_SANDBOX_TOOLS_DIR/rhai-run/bin/rhai-run" "$TIKEO_SANDBOX_TOOLS_DIR/rhai/bin/rhai-run"; \
    curl https://wasmtime.dev/install.sh -sSf | bash; cp "$HOME/.wasmtime/bin/wasmtime" "$TIKEO_SANDBOX_TOOLS_DIR/wasmtime/bin/wasmtime"; \
    rm -rf /tmp/deno.zip /tmp/pwsh.tar.gz "$HOME/.cargo/registry" "$HOME/.cargo/git"

WORKDIR /app
COPY target/app.jar /app/app.jar
ENTRYPOINT ["java", "-jar", "/app/app.jar"]
```

## Alpine Worker Dockerfile

Alpine is useful for small Go/Rust/Node/Python Workers. PowerShell on Alpine depends on compatibility libraries; validate `pwsh --version` in CI before publishing the image.

```docker
FROM alpine:3.22

ARG POWERSHELL_VERSION=7.5.4
ENV TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools \
    TIKEO_SANDBOX_STRICT_ISOLATION=1 \
    TIKEO_SANDBOX_AUTO_INSTALL=0

RUN apk add --no-cache \
      ca-certificates curl tar gzip unzip xz bash nodejs npm cargo ripgrep \
      icu-libs krb5-libs libgcc libintl libssl3 libstdc++ zlib

RUN set -eux; \
    npm install -g --prefix "$TIKEO_SANDBOX_TOOLS_DIR/srt" @anthropic-ai/sandbox-runtime; \
    cargo install --root "$TIKEO_SANDBOX_TOOLS_DIR/rhai-run" rhai --bins --features bin-features; \
    arch="$(apk --print-arch)"; \
    case "$arch" in x86_64) deno_arch=x86_64-unknown-linux-gnu; ps_arch=linux-x64 ;; aarch64) deno_arch=aarch64-unknown-linux-gnu; ps_arch=linux-arm64 ;; *) echo "unsupported arch: $arch"; exit 1 ;; esac; \
    curl -fsSL "https://github.com/denoland/deno/releases/latest/download/deno-${deno_arch}.zip" -o /tmp/deno.zip; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin"; unzip -q /tmp/deno.zip -d "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin"; chmod +x "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin/deno"; \
    curl -fsSL "https://github.com/PowerShell/PowerShell/releases/download/v${POWERSHELL_VERSION}/powershell-${POWERSHELL_VERSION}-${ps_arch}.tar.gz" -o /tmp/pwsh.tar.gz; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}" "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/bin"; \
    tar -xzf /tmp/pwsh.tar.gz -C "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}"; \
    chmod +x "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}/pwsh"; \
    ln -sf "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}/pwsh" "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/bin/pwsh"; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/sh/bin" "$TIKEO_SANDBOX_TOOLS_DIR/node/bin" "$TIKEO_SANDBOX_TOOLS_DIR/npm/bin" "$TIKEO_SANDBOX_TOOLS_DIR/rg/bin" "$TIKEO_SANDBOX_TOOLS_DIR/ripgrep/bin" "$TIKEO_SANDBOX_TOOLS_DIR/rhai/bin" "$TIKEO_SANDBOX_TOOLS_DIR/wasmtime/bin"; \
    ln -sf /bin/sh "$TIKEO_SANDBOX_TOOLS_DIR/sh/bin/sh"; \
    ln -sf "$(command -v node)" "$TIKEO_SANDBOX_TOOLS_DIR/node/bin/node"; \
    ln -sf "$(command -v npm)" "$TIKEO_SANDBOX_TOOLS_DIR/npm/bin/npm"; \
    ln -sf "$(command -v rg)" "$TIKEO_SANDBOX_TOOLS_DIR/rg/bin/rg"; \
    ln -sf "$(command -v rg)" "$TIKEO_SANDBOX_TOOLS_DIR/ripgrep/bin/rg"; \
    ln -sf "$TIKEO_SANDBOX_TOOLS_DIR/rhai-run/bin/rhai-run" "$TIKEO_SANDBOX_TOOLS_DIR/rhai/bin/rhai-run"; \
    curl https://wasmtime.dev/install.sh -sSf | bash; cp "$HOME/.wasmtime/bin/wasmtime" "$TIKEO_SANDBOX_TOOLS_DIR/wasmtime/bin/wasmtime"; \
    rm -rf /tmp/deno.zip /tmp/pwsh.tar.gz "$HOME/.cargo/registry" "$HOME/.cargo/git"

WORKDIR /app
COPY ./dist/worker /app/worker
ENTRYPOINT ["/app/worker"]
```

## Distroless / minimal runtime pattern

Distroless images cannot run package managers in the final stage. Build sandbox tools in a normal builder stage, copy only `/opt/tikeo/sandbox-tools`, and keep strict mode enabled.

```docker
FROM debian:bookworm-slim AS sandbox-tools
ARG POWERSHELL_VERSION=7.5.4
ENV TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl tar gzip unzip bash nodejs npm cargo ripgrep && rm -rf /var/lib/apt/lists/*
RUN set -eux; \
    npm install -g --prefix "$TIKEO_SANDBOX_TOOLS_DIR/srt" @anthropic-ai/sandbox-runtime; \
    cargo install --root "$TIKEO_SANDBOX_TOOLS_DIR/rhai-run" rhai --bins --features bin-features; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin" "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/bin" "$TIKEO_SANDBOX_TOOLS_DIR/rg/bin" "$TIKEO_SANDBOX_TOOLS_DIR/ripgrep/bin" "$TIKEO_SANDBOX_TOOLS_DIR/node/bin" "$TIKEO_SANDBOX_TOOLS_DIR/npm/bin" "$TIKEO_SANDBOX_TOOLS_DIR/sh/bin"; \
    curl -fsSL "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip" -o /tmp/deno.zip; \
    unzip -q /tmp/deno.zip -d "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin"; \
    curl -fsSL "https://github.com/PowerShell/PowerShell/releases/download/v${POWERSHELL_VERSION}/powershell-${POWERSHELL_VERSION}-linux-x64.tar.gz" -o /tmp/pwsh.tar.gz; \
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}"; \
    tar -xzf /tmp/pwsh.tar.gz -C "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}"; \
    chmod +x "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}/pwsh" "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin/deno"; \
    ln -sf "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}/pwsh" "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/bin/pwsh"; \
    ln -sf /bin/sh "$TIKEO_SANDBOX_TOOLS_DIR/sh/bin/sh"; \
    ln -sf "$(command -v node)" "$TIKEO_SANDBOX_TOOLS_DIR/node/bin/node"; \
    ln -sf "$(command -v npm)" "$TIKEO_SANDBOX_TOOLS_DIR/npm/bin/npm"; \
    ln -sf "$(command -v rg)" "$TIKEO_SANDBOX_TOOLS_DIR/rg/bin/rg"; \
    ln -sf "$(command -v rg)" "$TIKEO_SANDBOX_TOOLS_DIR/ripgrep/bin/rg"

FROM gcr.io/distroless/java21-debian12
ENV TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools \
    TIKEO_SANDBOX_STRICT_ISOLATION=1 \
    TIKEO_SANDBOX_AUTO_INSTALL=0
COPY --from=sandbox-tools /opt/tikeo/sandbox-tools /opt/tikeo/sandbox-tools
COPY target/app.jar /app/app.jar
ENTRYPOINT ["java", "-jar", "/app/app.jar"]
```

## Host or VM install shape

On a VM or bare-metal Worker host, use the same layout. Example for Debian/Ubuntu:

```bash
sudo install -d -m 0755 /opt/tikeo/sandbox-tools
sudo apt-get update
sudo apt-get install -y ca-certificates curl tar gzip unzip bash nodejs npm cargo ripgrep
sudo npm install -g --prefix /opt/tikeo/sandbox-tools/srt @anthropic-ai/sandbox-runtime
sudo cargo install --root /opt/tikeo/sandbox-tools/rhai-run rhai --bins --features bin-features
curl -fsSL https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip -o /tmp/deno.zip
sudo install -d /opt/tikeo/sandbox-tools/deno/bin
sudo unzip -o /tmp/deno.zip -d /opt/tikeo/sandbox-tools/deno/bin
sudo chmod +x /opt/tikeo/sandbox-tools/deno/bin/deno
```

Then set the Worker service environment:

```properties
Environment=TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools
Environment=TIKEO_SANDBOX_STRICT_ISOLATION=1
Environment=TIKEO_SANDBOX_AUTO_INSTALL=0
```

## Validation checklist

Run these inside the final image or host before declaring script capabilities available:

```bash
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/srt/bin/srt --help || true
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/rg/bin/rg --version
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/ripgrep/bin/rg --version
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/deno/bin/deno --version
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/rhai-run/bin/rhai-run --help || true
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/pwsh/bin/pwsh --version
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/wasmtime/bin/wasmtime --version
```

If a tool is absent, the SDK must not advertise its corresponding script capability. If a task still reaches that runner, it fails closed and writes a clear diagnostic instead of stopping the business process.
