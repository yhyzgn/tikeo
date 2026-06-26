---
title: Worker 沙箱工具与 Dockerfile
description: 在宿主机或 Worker 镜像中预装 Tikeo 沙箱工具，支持宿主 PATH 复用和严格沙箱隔离。
keywords: [tikeo worker, sandbox tools, dockerfile, powershell, deno, srt, wasmtime]
---

# Worker 沙箱工具与 Dockerfile

Tikeo Worker 可以执行 Normal Processor，也可以按需执行脚本/插件运行器。脚本运行器可能需要 SRT、Deno、ripgrep、Rhai、PowerShell、Wasmtime、WasmEdge 等本地工具。

生产环境不要依赖“启动时下载工具”。更稳妥的做法是把 Worker 宿主机或镜像提前准备好，只声明已经可用的能力。

## 运行模式

| 模式 | 配置 | 工具查找行为 | 推荐场景 |
| --- | --- | --- | --- |
| 默认宿主复用 | `TIKEO_SANDBOX_STRICT_ISOLATION=0` 或不设置 | SDK 优先使用宿主 `PATH` 中可用的二进制，然后查 legacy `state-dir/sandbox-tools`，最后查 `TIKEO_SANDBOX_TOOLS_DIR` / `~/.tikeo/sandbox-tools`。任务仍会使用 sandbox `cwd`、`HOME`、`TMPDIR`、`DENO_DIR` 和 PowerShell/.NET 缓存目录。 | 大多数可信 Worker 镜像。 |
| 严格沙箱隔离 | `TIKEO_SANDBOX_STRICT_ISOLATION=1`；Java Boot 也可用 `tikeo.worker.scripts.strict-sandbox-isolation=true` | SDK 跳过宿主 `PATH` 工具和解释器，只使用 `TIKEO_SANDBOX_TOOLS_DIR` / `~/.tikeo/sandbox-tools` 下的二进制。缺失工具不会声明能力，并 fail-closed。 | 多作用域、强合规、变更受控 Worker。 |
| 本地/demo 自动预热 | `TIKEO_SANDBOX_AUTO_INSTALL=1` / SDK 默认值 | 缺失工具只触发后台预热，启动不会等待下载完成。 | 开发机、demo、临时 smoke test。 |

生产镜像建议设置：

```bash
TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools
TIKEO_SANDBOX_AUTO_INSTALL=0
# 如需更强隔离：
TIKEO_SANDBOX_STRICT_ISOLATION=1
```

## 工具来源清单

| 工具 / 二进制 | 用途 | 是否能从发行版/中央仓库安装 | 需要手动安装时的来源 |
| --- | --- | --- | --- |
| `sh`, `bash` | Shell 脚本运行器和安装脚本 | `apt`、`dnf`、`apk` | 通常无需手动安装；严格模式可把 `/bin/sh` 链接进缓存。 |
| `node`, `npm` | SRT launcher 与 npm 安装工具 | 多数发行版仓库、NodeSource、官方 Node 镜像 | 发行版版本太旧时用官方 Node 二进制包。 |
| `srt` | Anthropic Sandbox Runtime 后端 | npm registry | `npm install -g --prefix /opt/tikeo/sandbox-tools/srt @anthropic-ai/sandbox-runtime`。 |
| `rg` | SRT 依赖的 ripgrep | Debian/Ubuntu/Fedora/RHEL/Alpine 包；crates.io | `cargo install --root /opt/tikeo/sandbox-tools/rg ripgrep`；Java 也可使用 `/opt/tikeo/sandbox-tools/ripgrep/bin/rg`。 |
| `deno` | JavaScript/TypeScript 沙箱执行 | 基础发行版仓库中不稳定 | 官方安装脚本或 GitHub release zip，放到 `/opt/tikeo/sandbox-tools/deno/bin/deno`。 |
| `rhai-run` | Rhai 脚本执行 | crates.io | `cargo install --root /opt/tikeo/sandbox-tools/rhai-run rhai --bins --features bin-features`；Java 也可使用 `/opt/tikeo/sandbox-tools/rhai/bin/rhai-run`。 |
| `pwsh` | PowerShell 脚本 | Microsoft 支持 Debian/Ubuntu/RHEL 系包源；Alpine 通常用 tarball | 从 PowerShell GitHub Releases 下载 `powershell-${version}-linux-x64.tar.gz` 或 `linux-arm64.tar.gz`，解压到 `/opt/tikeo/sandbox-tools/pwsh`。 |
| `wasmtime` | WASM 运行时 | 通常用 upstream installer 或 release archive；也可 cargo fallback | `curl https://wasmtime.dev/install.sh -sSf | bash` 后复制/链接到缓存。 |
| `wasmedge` | 可选 WasmEdge 后端 | Fedora/EPEL 等可能有包，否则用官方脚本 | `curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash` 后复制/链接到缓存。 |

:::tip 多 SDK 镜像
当前不同 SDK 的缓存 key 略有差异。建议同时创建 `rg` 和 `ripgrep`、`rhai-run` 和 `rhai` 两组目录，避免 Java 与 Go/Python/Node/Rust 镜像分叉。
:::

## 官方安装参考

版本固定或改造成企业内网镜像源时，建议以这些上游页面为准：

- PowerShell：[Linux 支持概览](https://learn.microsoft.com/en-us/powershell/scripting/install/linux-overview) 和 [tar.gz 归档安装](https://learn.microsoft.com/en-us/powershell/scripting/install/alternate-install-methods)。
- Deno：[Installation](https://docs.deno.com/runtime/getting_started/installation/)。
- Wasmtime：[CLI installation](https://docs.wasmtime.dev/cli-install.html)。
- WasmEdge：[Install and uninstall WasmEdge](https://wasmedge.org/docs/start/install/)。

## 严格模式推荐缓存布局

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

## Debian / Ubuntu Worker Dockerfile

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
    mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin"; unzip -q /tmp/deno.zip -d "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin"; chmod +x "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin/deno"; \
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
    curl https://wasmtime.dev/install.sh -sSf | bash; mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/wasmtime/bin"; cp "$HOME/.wasmtime/bin/wasmtime" "$TIKEO_SANDBOX_TOOLS_DIR/wasmtime/bin/wasmtime"; \
    rm -rf /tmp/deno.zip /tmp/pwsh.tar.gz "$HOME/.cargo/registry" "$HOME/.cargo/git"

WORKDIR /app
COPY target/app.jar /app/app.jar
ENTRYPOINT ["java", "-jar", "/app/app.jar"]
```

## RHEL / UBI / Fedora Worker Dockerfile

下面示例使用 Fedora，因为所需构建期包可直接从默认仓库安装。UBI/RHEL minimal 镜像如果缺少 `cargo` / `ripgrep`，需要启用对应 Red Hat 仓库，或使用下方 Distroless 风格的 builder stage，把最终工具缓存复制到运行时镜像。

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
    ln -sf "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}/pwsh" "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/bin/pwsh"

WORKDIR /app
COPY target/app.jar /app/app.jar
ENTRYPOINT ["java", "-jar", "/app/app.jar"]
```

## Alpine Worker Dockerfile

Alpine 适合 Go/Rust/Node/Python Worker。PowerShell 在 Alpine 上依赖兼容库，发布镜像前务必在 CI 中执行 `pwsh --version`。

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

## Distroless / 极简运行时

Distroless 最终镜像不能运行包管理器。应在 builder stage 准备 `/opt/tikeo/sandbox-tools`，最终阶段只复制工具目录并开启严格模式。

```docker
FROM debian:bookworm-slim AS sandbox-tools
ARG POWERSHELL_VERSION=7.5.4
ENV TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl tar gzip unzip bash nodejs npm cargo ripgrep && rm -rf /var/lib/apt/lists/*
RUN npm install -g --prefix "$TIKEO_SANDBOX_TOOLS_DIR/srt" @anthropic-ai/sandbox-runtime \
 && cargo install --root "$TIKEO_SANDBOX_TOOLS_DIR/rhai-run" rhai --bins --features bin-features \
 && mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin" "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/bin" "$TIKEO_SANDBOX_TOOLS_DIR/rg/bin" "$TIKEO_SANDBOX_TOOLS_DIR/ripgrep/bin" "$TIKEO_SANDBOX_TOOLS_DIR/node/bin" "$TIKEO_SANDBOX_TOOLS_DIR/npm/bin" "$TIKEO_SANDBOX_TOOLS_DIR/sh/bin" \
 && curl -fsSL "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip" -o /tmp/deno.zip \
 && unzip -q /tmp/deno.zip -d "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin" \
 && curl -fsSL "https://github.com/PowerShell/PowerShell/releases/download/v${POWERSHELL_VERSION}/powershell-${POWERSHELL_VERSION}-linux-x64.tar.gz" -o /tmp/pwsh.tar.gz \
 && mkdir -p "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}" \
 && tar -xzf /tmp/pwsh.tar.gz -C "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}" \
 && chmod +x "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}/pwsh" "$TIKEO_SANDBOX_TOOLS_DIR/deno/bin/deno" \
 && ln -sf "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/powershell-${POWERSHELL_VERSION}/pwsh" "$TIKEO_SANDBOX_TOOLS_DIR/pwsh/bin/pwsh" \
 && ln -sf /bin/sh "$TIKEO_SANDBOX_TOOLS_DIR/sh/bin/sh" \
 && ln -sf "$(command -v node)" "$TIKEO_SANDBOX_TOOLS_DIR/node/bin/node" \
 && ln -sf "$(command -v npm)" "$TIKEO_SANDBOX_TOOLS_DIR/npm/bin/npm" \
 && ln -sf "$(command -v rg)" "$TIKEO_SANDBOX_TOOLS_DIR/rg/bin/rg" \
 && ln -sf "$(command -v rg)" "$TIKEO_SANDBOX_TOOLS_DIR/ripgrep/bin/rg"

FROM gcr.io/distroless/java21-debian12
ENV TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools \
    TIKEO_SANDBOX_STRICT_ISOLATION=1 \
    TIKEO_SANDBOX_AUTO_INSTALL=0
COPY --from=sandbox-tools /opt/tikeo/sandbox-tools /opt/tikeo/sandbox-tools
COPY target/app.jar /app/app.jar
ENTRYPOINT ["java", "-jar", "/app/app.jar"]
```

## 宿主机 / VM 安装方式

宿主机也使用同样目录。Debian/Ubuntu 示例：

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

Worker service 环境变量：

```properties
Environment=TIKEO_SANDBOX_TOOLS_DIR=/opt/tikeo/sandbox-tools
Environment=TIKEO_SANDBOX_STRICT_ISOLATION=1
Environment=TIKEO_SANDBOX_AUTO_INSTALL=0
```

## 验收清单

在最终镜像或宿主机上执行：

```bash
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/srt/bin/srt --help || true
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/rg/bin/rg --version
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/ripgrep/bin/rg --version
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/deno/bin/deno --version
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/rhai-run/bin/rhai-run --help || true
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/pwsh/bin/pwsh --version
${TIKEO_SANDBOX_TOOLS_DIR:-/opt/tikeo/sandbox-tools}/wasmtime/bin/wasmtime --version
```

如果某个工具不可用，SDK 不应声明对应脚本能力。任务仍然命中该 runner 时，会 fail-closed 并写出明确诊断，而不是让业务进程停止。
