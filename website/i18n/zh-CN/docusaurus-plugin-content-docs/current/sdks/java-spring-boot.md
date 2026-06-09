---
title: Java Spring Boot Starter
description: Java SDK artifact、依赖选择、Spring Boot 配置、环境变量与部署清单。
---

# Java Spring Boot Starter

Java SDK 以 Maven Central artifact 发布，group 为 `net.tikeo`。每个服务应该只添加 **一个** Tikeo 依赖：普通 Java SDK、匹配的 Spring Boot starter，或一个高级 Spring Framework adapter。不要显式添加所选依赖已经传递带入的上游 Tikeo 模块。

## 运行时与版本占位符

- Java runtime：Java 17+。
- 仓库 CI 使用 Temurin 21 验证 SDK。
- 将 `<TIKEO_VERSION>` 替换为 README 顶部对应 artifact/package 徽标显示的版本号。
- Maven Central 不使用 Go 的 `v<TIKEO_VERSION>` tag 形式；Java 依赖使用不带 `v` 的 `<TIKEO_VERSION>`。

## 只选择一个 Java artifact

| Artifact | 什么时候使用 | 依赖行 |
| --- | --- | --- |
| `net.tikeo:tikeo` | 普通 Java Worker、management client、sandbox tooling 或低层 Worker Tunnel 集成。 | `implementation("net.tikeo:tikeo:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring-boot-starter` | Spring Boot 4 / Spring Framework 7 应用，需要自动配置。 | `implementation("net.tikeo:tikeo-spring-boot-starter:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring-boot3-starter` | Spring Boot 3 / Spring Framework 6 应用，需要自动配置。 | `implementation("net.tikeo:tikeo-spring-boot3-starter:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring-boot2-starter` | Spring Boot 2 / Spring Framework 5 应用，需要自动配置。 | `implementation("net.tikeo:tikeo-spring-boot2-starter:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring` | 不使用 Boot starter，手动接线 Spring Framework 7 adapter。 | `implementation("net.tikeo:tikeo-spring:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring6` | 不使用 Boot starter，手动接线 Spring Framework 6 adapter。 | `implementation("net.tikeo:tikeo-spring6:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring5` | 不使用 Boot starter，手动接线 Spring Framework 5 adapter。 | `implementation("net.tikeo:tikeo-spring5:<TIKEO_VERSION>")` |

Spring Boot starter 会传递包含匹配的 Spring adapter 和 core SDK。例如 Spring Boot 3 服务只需要依赖 `tikeo-spring-boot3-starter`；除非你在刻意覆盖依赖解析，否则不要再额外声明 `tikeo-spring6` 或 `tikeo`。

## Gradle Kotlin DSL

```kotlin
repositories {
    mavenCentral()
}

dependencies {
    // 只选择一个：
    implementation("net.tikeo:tikeo:<TIKEO_VERSION>")                    // plain Java
    // implementation("net.tikeo:tikeo-spring-boot-starter:<TIKEO_VERSION>")  // Spring Boot 4
    // implementation("net.tikeo:tikeo-spring-boot3-starter:<TIKEO_VERSION>") // Spring Boot 3
    // implementation("net.tikeo:tikeo-spring-boot2-starter:<TIKEO_VERSION>") // Spring Boot 2

    // 高级 adapter；需要时也只选择一个：
    // implementation("net.tikeo:tikeo-spring:<TIKEO_VERSION>")  // Spring Framework 7
    // implementation("net.tikeo:tikeo-spring6:<TIKEO_VERSION>") // Spring Framework 6
    // implementation("net.tikeo:tikeo-spring5:<TIKEO_VERSION>") // Spring Framework 5
}
```

## Maven

使用一个 dependency block，并从上表替换 `artifactId`：

```xml
<dependency>
  <groupId>net.tikeo</groupId>
  <artifactId>tikeo-spring-boot3-starter</artifactId>
  <version>&lt;TIKEO_VERSION&gt;</version>
</dependency>
```

常用 `artifactId`：

- `tikeo` — 普通 Java。
- `tikeo-spring-boot-starter` — Spring Boot 4。
- `tikeo-spring-boot3-starter` — Spring Boot 3。
- `tikeo-spring-boot2-starter` — Spring Boot 2。
- `tikeo-spring`、`tikeo-spring6`、`tikeo-spring5` — 高级手动 Spring Framework adapter。

## 最小 Spring Boot Worker 配置

`endpoint` 要设置成 Worker 进程能访问到的 Worker Tunnel 地址。本地 demo 使用 `127.0.0.1`；Kubernetes/VM 部署通常使用 Service、负载均衡或内网 DNS。

```yaml
tikeo:
  worker:
    enabled: true
    auto-startup: true
    dry-run: false
    endpoint: ${TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}
    namespace: ${TIKEO_WORKER_NAMESPACE:default}
    app: ${TIKEO_WORKER_APP:default}
    cluster: ${TIKEO_WORKER_CLUSTER:default}
    region: ${TIKEO_WORKER_REGION:default}
    capabilities:
      - java
      - spring-boot
    labels:
      worker_pool: ${TIKEO_WORKER_POOL:java-blue}
      runtime: java
```

## 完整 Spring Boot 配置模板

```yaml
server:
  port: ${TIKEO_DEMO_SERVER_PORT:18083}

tikeo:
  worker:
    enabled: ${TIKEO_WORKER_ENABLED:true}
    auto-startup: ${TIKEO_WORKER_AUTO_STARTUP:true}
    dry-run: ${TIKEO_WORKER_DRY_RUN:false}
    endpoint: ${TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}
    heartbeat-interval-millis: ${TIKEO_WORKER_HEARTBEAT_INTERVAL_MILLIS:10000}
    client-instance-id: ${TIKEO_WORKER_CLIENT_INSTANCE_ID:}
    state-dir: ${TIKEO_WORKER_STATE_DIR:}
    namespace: ${TIKEO_WORKER_NAMESPACE:default}
    app: ${TIKEO_WORKER_APP:default}
    cluster: ${TIKEO_WORKER_CLUSTER:default}
    region: ${TIKEO_WORKER_REGION:default}
    capabilities:
      - java
      - spring-boot
    labels:
      worker_pool: ${TIKEO_WORKER_POOL:java-blue}
      runtime: java
    election:
      enabled: ${TIKEO_WORKER_ELECTION_ENABLED:true}
      domain: ${TIKEO_WORKER_ELECTION_DOMAIN:}
      priority: ${TIKEO_WORKER_ELECTION_PRIORITY:100}
    wasm:
      auto-install: ${TIKEO_WORKER_WASM_AUTO_INSTALL:true}
      install-version: ${TIKEO_WORKER_WASM_VERSION:latest}
      install-dir: ${TIKEO_WORKER_WASM_INSTALL_DIR:}
      installer-url: ${TIKEO_WORKER_WASM_INSTALLER_URL:https://wasmtime.dev/install.sh}
      install-timeout-millis: ${TIKEO_WORKER_WASM_INSTALL_TIMEOUT_MILLIS:120000}
    scripts:
      enabled: ${TIKEO_WORKER_SCRIPTS_ENABLED:true}
      container-enabled: ${TIKEO_WORKER_CONTAINER_SCRIPTS_ENABLED:false}
      availability-check: ${TIKEO_WORKER_SCRIPT_RUNTIME_CHECK:true}
      runtime-command: ${TIKEO_WORKER_CONTAINER_RUNTIME:}
      runtime-args: []
      auto-install-tools: ${TIKEO_WORKER_SCRIPT_AUTO_INSTALL_TOOLS:true}
      srt-install-version: ${TIKEO_WORKER_SCRIPT_SRT_VERSION:latest}
      srt-install-dir: ${TIKEO_WORKER_SCRIPT_SRT_INSTALL_DIR:}
      ripgrep-install-version: ${TIKEO_WORKER_SCRIPT_RIPGREP_VERSION:latest}
      ripgrep-install-dir: ${TIKEO_WORKER_SCRIPT_RIPGREP_INSTALL_DIR:}
      deno-install-version: ${TIKEO_WORKER_SCRIPT_DENO_VERSION:latest}
      deno-install-dir: ${TIKEO_WORKER_SCRIPT_DENO_INSTALL_DIR:}
      deno-installer-url: ${TIKEO_WORKER_SCRIPT_DENO_INSTALLER_URL:https://deno.land/install.sh}
      rhai-install-version: ${TIKEO_WORKER_SCRIPT_RHAI_VERSION:}
      rhai-install-dir: ${TIKEO_WORKER_SCRIPT_RHAI_INSTALL_DIR:}
      power-shell-install-version: ${TIKEO_WORKER_SCRIPT_POWERSHELL_VERSION:7.5.4}
      power-shell-install-dir: ${TIKEO_WORKER_SCRIPT_POWERSHELL_INSTALL_DIR:}
      wasmedge-auto-install: ${TIKEO_WORKER_SCRIPT_WASMEDGE_AUTO_INSTALL:false}
      wasmedge-install-version: ${TIKEO_WORKER_SCRIPT_WASMEDGE_VERSION:latest}
      wasmedge-install-dir: ${TIKEO_WORKER_SCRIPT_WASMEDGE_INSTALL_DIR:}
      wasmedge-installer-url: ${TIKEO_WORKER_SCRIPT_WASMEDGE_INSTALLER_URL:https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh}
      v8-install-version: ${TIKEO_WORKER_SCRIPT_V8_VERSION:latest}
      v8-install-dir: ${TIKEO_WORKER_SCRIPT_V8_INSTALL_DIR:}
      tool-install-timeout-millis: ${TIKEO_WORKER_SCRIPT_TOOL_INSTALL_TIMEOUT_MILLIS:120000}
      images:
        shell: ${TIKEO_WORKER_SCRIPT_SHELL_IMAGE:}
        python: ${TIKEO_WORKER_SCRIPT_PYTHON_IMAGE:}
        js: ${TIKEO_WORKER_SCRIPT_JAVASCRIPT_IMAGE:}
        ts: ${TIKEO_WORKER_SCRIPT_TYPESCRIPT_IMAGE:}
        powershell: ${TIKEO_WORKER_SCRIPT_POWERSHELL_IMAGE:}
        php: ${TIKEO_WORKER_SCRIPT_PHP_IMAGE:}
        groovy: ${TIKEO_WORKER_SCRIPT_GROOVY_IMAGE:}
        rhai: ${TIKEO_WORKER_SCRIPT_RHAI_IMAGE:}

  management:
    enabled: ${TIKEO_MANAGEMENT_ENABLED:false}
    endpoint: ${TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9999}
    api-key: ${TIKEO_MANAGEMENT_API_KEY:}
    namespace: ${TIKEO_MANAGEMENT_NAMESPACE:default}
    app: ${TIKEO_MANAGEMENT_APP:default}
```

## Worker 配置项与默认值

| 配置项 | 默认值 | 生产环境建议 |
| --- | --- | --- |
| `tikeo.worker.enabled` | `true` | Worker 服务保持启用；仅在不应注册 Worker 的 profile 中关闭。 |
| `tikeo.worker.auto-startup` | `true` | 除非手动控制生命周期，否则保持启用。 |
| `tikeo.worker.endpoint` | `http://0.0.0.0:9998` | 显式设置为当前服务能访问到的 Worker Tunnel endpoint。 |
| `tikeo.worker.dry-run` | `false` | live worker 保持 `false`；配置冒烟测试可设为 `true`。 |
| `tikeo.worker.heartbeat-interval-millis` | `10000` | 仅在高延迟网络中调整，并验证故障检测行为。 |
| `tikeo.worker.client-instance-id` | 空 | 可选。副本场景优先使用空值 + 持久化 `state-dir` 生成身份。 |
| `tikeo.worker.state-dir` | 空 → `~/.tikeo/workers` | 需要稳定生成 instance id 时持久化该路径。 |
| `tikeo.worker.namespace` | `default` | 设置为租户/环境 namespace。 |
| `tikeo.worker.app` | `default` | 设置为路由和 management scope 使用的 app 边界。 |
| `tikeo.worker.cluster` | `default` | 设置为集群、环境或 worker pool 名称。 |
| `tikeo.worker.region` | `default` | 设置为 region/zone。 |
| `tikeo.worker.capabilities` | `[]` | 添加 `java`、`spring-boot`、`billing`、`reports` 等路由能力。 |
| `tikeo.worker.labels` | `{}` | 添加 `worker_pool`、`runtime`、`team`、`tier` 等运维标签。 |
| `tikeo.worker.election.enabled` | `true` | 除非服务永远不应成为 leader，否则保持启用。 |
| `tikeo.worker.election.domain` | 空 | 空表示 `namespace/app/cluster/region`；多个逻辑池共用这些值时显式设置。 |
| `tikeo.worker.election.priority` | `100` | 数值越小越优先；用固定值表达确定性的 leader 倾向。 |

## Sandbox 与脚本工具默认值

| 配置项 | 默认值 | 说明 |
| --- | --- | --- |
| `tikeo.worker.wasm.auto-install` | `true` | 缺少 Wasmtime 时自动安装。不可变生产镜像中建议关闭。 |
| `tikeo.worker.wasm.install-version` | `latest` | Wasmtime installer 版本，例如 `latest` 或 `v45.0.0`。 |
| `tikeo.worker.wasm.install-dir` | 空 → `~/.tikeo/sandbox-tools/wasmtime` | 持久化/缓存该目录，避免重复下载。 |
| `tikeo.worker.wasm.installer-url` | `https://wasmtime.dev/install.sh` | 可覆盖为内部镜像。 |
| `tikeo.worker.wasm.install-timeout-millis` | `120000` | 安装超时。 |
| `tikeo.worker.scripts.enabled` | `true` | 启用默认 sandbox 路径的动态脚本执行。 |
| `tikeo.worker.scripts.container-enabled` | `false` | 启用可选 Docker/Podman 语言 runner。 |
| `tikeo.worker.scripts.availability-check` | `true` | 广播 capability 前探测工具。 |
| `tikeo.worker.scripts.runtime-command` | 空 | 启用容器脚本时设置为 `docker`、`podman` 或兼容 runtime。 |
| `tikeo.worker.scripts.runtime-args` | `[]` | 追加在 image 前面的 runtime 参数。 |
| `tikeo.worker.scripts.auto-install-tools` | `true` | 缺少本地开发工具时自动安装。生产锁定主机建议关闭。 |
| `tikeo.worker.scripts.srt-install-version` | `latest` | Anthropic Sandbox Runtime npm package 版本。 |
| `tikeo.worker.scripts.srt-install-dir` | 空 → `~/.tikeo/sandbox-tools/srt` | SRT 安装/缓存目录。 |
| `tikeo.worker.scripts.ripgrep-install-version` | `latest` | SRT 需要的 ripgrep 版本。 |
| `tikeo.worker.scripts.ripgrep-install-dir` | 空 → `~/.tikeo/sandbox-tools/ripgrep` | ripgrep 安装/缓存目录。 |
| `tikeo.worker.scripts.deno-install-version` | `latest` | Deno installer 版本。 |
| `tikeo.worker.scripts.deno-install-dir` | 空 → `~/.tikeo/sandbox-tools/deno` | Deno 安装/缓存目录。 |
| `tikeo.worker.scripts.deno-installer-url` | `https://deno.land/install.sh` | 可覆盖为内部镜像。 |
| `tikeo.worker.scripts.rhai-install-version` | 空 | 空表示最新可 cargo install 的 Rhai tooling。 |
| `tikeo.worker.scripts.rhai-install-dir` | 空 → `~/.tikeo/sandbox-tools/rhai` | Rhai 安装/缓存目录。 |
| `tikeo.worker.scripts.power-shell-install-version` | `7.5.4` | SRT-backed PowerShell 使用的 PowerShell Core 版本。 |
| `tikeo.worker.scripts.power-shell-install-dir` | 空 → `~/.tikeo/sandbox-tools/pwsh` | PowerShell 安装/缓存目录。持久化/缓存它可避免重复下载 archive。 |
| `tikeo.worker.scripts.wasmedge-auto-install` | `false` | 默认关闭，显式选择后才安装。 |
| `tikeo.worker.scripts.wasmedge-install-version` | `latest` | WasmEdge installer 版本。 |
| `tikeo.worker.scripts.wasmedge-install-dir` | 空 → `~/.tikeo/sandbox-tools/wasmedge` | WasmEdge 安装/缓存目录。 |
| `tikeo.worker.scripts.wasmedge-installer-url` | `https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh` | 可覆盖为内部镜像。 |
| `tikeo.worker.scripts.v8-install-version` | `latest` | V8 runtime 版本。 |
| `tikeo.worker.scripts.v8-install-dir` | 空 → `~/.tikeo/sandbox-tools/v8` | V8 安装/缓存目录。 |
| `tikeo.worker.scripts.tool-install-timeout-millis` | `120000` | 工具安装超时。 |
| `tikeo.worker.scripts.images.shell` | 空 | Shell 脚本容器镜像；为空禁用该镜像。 |
| `tikeo.worker.scripts.images.python` | 空 | Python 脚本容器镜像；为空禁用该镜像。 |
| `tikeo.worker.scripts.images.js` | 空 | JavaScript 脚本容器镜像；为空禁用该镜像。 |
| `tikeo.worker.scripts.images.ts` | 空 | TypeScript 脚本容器镜像；为空禁用该镜像。 |
| `tikeo.worker.scripts.images.powershell` | 空 | PowerShell 脚本容器镜像；为空禁用该镜像。 |
| `tikeo.worker.scripts.images.php` | 空 | PHP 脚本容器镜像；为空禁用该镜像。 |
| `tikeo.worker.scripts.images.groovy` | 空 | Groovy 脚本容器镜像；为空禁用该镜像。 |
| `tikeo.worker.scripts.images.rhai` | 空 | Rhai 脚本容器镜像；为空禁用该镜像。 |

## Management client 配置项

| 配置项 | 默认值 | 生产环境建议 |
| --- | --- | --- |
| `tikeo.management.enabled` | `false` | 只在需要 management/control-plane SDK client 的服务中启用。 |
| `tikeo.management.endpoint` | `http://127.0.0.1:9999` | 显式设置。Compose 示例通常将 server HTTP 暴露在 `9090`。 |
| `tikeo.management.api-key` | 空 | 将 app-scoped API key 放入 Secret store，再以环境变量注入。 |
| `tikeo.management.namespace` | `default` | 将 management 操作限制到目标 namespace。 |
| `tikeo.management.app` | `default` | 将 management 操作限制到目标 app。 |

## 示例使用的环境变量

Spring Boot demo 会把环境变量映射到 `application.yml`。生产中可以沿用这些名称，也可以将平台配置名称映射到上面的 Spring 配置项。

| 环境变量 | 映射到 | 示例默认值 |
| --- | --- | --- |
| `TIKEO_DEMO_SERVER_PORT` | `server.port` | 根据 demo 为 `18082`、`18083` 或 `18084`。 |
| `TIKEO_WORKER_DRY_RUN` | `tikeo.worker.dry-run` | `false` |
| `TIKEO_WORKER_ENDPOINT` | `tikeo.worker.endpoint` | `http://127.0.0.1:9998` |
| `TIKEO_WORKER_CLIENT_INSTANCE_ID` | `tikeo.worker.client-instance-id` | 使用 `${HOSTNAME}` fallback 的 demo-specific 值。 |
| `TIKEO_WORKER_STATE_DIR` | `tikeo.worker.state-dir` | 空 |
| `TIKEO_WORKER_NAMESPACE` | `tikeo.worker.namespace` | 根据 demo 为 `default` 或 `dev-alpha`。 |
| `TIKEO_WORKER_APP` | `tikeo.worker.app` | 根据 demo 为 `default`、`orders` 或 `billing`。 |
| `TIKEO_WORKER_CLUSTER` | `tikeo.worker.cluster` | `local` |
| `TIKEO_WORKER_REGION` | `tikeo.worker.region` | `local` |
| `TIKEO_WORKER_POOL` | `tikeo.worker.labels.worker_pool` | `boot2-blue`、`boot3-blue` 或 `boot4-green` |
| `TIKEO_WORKER_WASM_AUTO_INSTALL` | `tikeo.worker.wasm.auto-install` | `true` |
| `TIKEO_WORKER_WASM_VERSION` | `tikeo.worker.wasm.install-version` | `latest` |
| `TIKEO_WORKER_WASM_INSTALL_DIR` | `tikeo.worker.wasm.install-dir` | 空 |
| `TIKEO_WORKER_SCRIPTS_ENABLED` | `tikeo.worker.scripts.enabled` | `true` |
| `TIKEO_WORKER_CONTAINER_SCRIPTS_ENABLED` | `tikeo.worker.scripts.container-enabled` | `false` |
| `TIKEO_WORKER_CONTAINER_RUNTIME` | `tikeo.worker.scripts.runtime-command` | 空 |
| `TIKEO_WORKER_SCRIPT_RUNTIME_CHECK` | `tikeo.worker.scripts.availability-check` | `true` |
| `TIKEO_WORKER_SCRIPT_AUTO_INSTALL_TOOLS` | `tikeo.worker.scripts.auto-install-tools` | `true` |
| `TIKEO_WORKER_SCRIPT_SHELL_IMAGE` | `tikeo.worker.scripts.images.shell` | 空 |
| `TIKEO_WORKER_SCRIPT_PYTHON_IMAGE` | `tikeo.worker.scripts.images.python` | 空 |
| `TIKEO_WORKER_SCRIPT_JAVASCRIPT_IMAGE` | `tikeo.worker.scripts.images.js` | 空 |
| `TIKEO_WORKER_SCRIPT_TYPESCRIPT_IMAGE` | `tikeo.worker.scripts.images.ts` | 空 |
| `TIKEO_WORKER_SCRIPT_POWERSHELL_IMAGE` | `tikeo.worker.scripts.images.powershell` | 空 |
| `TIKEO_WORKER_SCRIPT_RHAI_IMAGE` | `tikeo.worker.scripts.images.rhai` | 空 |
| `TIKEO_MANAGEMENT_ENABLED` | `tikeo.management.enabled` | demo 中为 `true`，starter 源码默认值为 `false`。 |
| `TIKEO_MANAGEMENT_ENDPOINT` | `tikeo.management.endpoint` | `http://127.0.0.1:9999` |
| `TIKEO_MANAGEMENT_API_KEY` | `tikeo.management.api-key` | 仅用于 demo 的 token；真实部署必须替换。 |
| `TIKEO_MANAGEMENT_NAMESPACE` | `tikeo.management.namespace` | `dev-alpha` |
| `TIKEO_MANAGEMENT_APP` | `tikeo.management.app` | `orders` 或 `billing` |

低层 Java SDK identity 与 installer 环境变量：

| 环境变量 | 说明 |
| --- | --- |
| `TIKEO_WORKER_RUNTIME_ID` | 生成 client instance id 时使用的 runtime identity 输入。 |
| `TIKEO_POD_NAME`、`POD_NAME`、`HOSTNAME` | 生成 worker instance id 的 fallback runtime identity 输入。 |
| `TIKEO_POWERSHELL_VERSION` | PowerShell installer fallback 版本；默认 `7.5.4`。 |
| `TIKEO_POWERSHELL_DOWNLOAD_URL` | 离线/镜像环境下覆盖 PowerShell archive URL。 |
| `TIKEO_POWERSHELL_INSTALL_TIMEOUT_MILLIS` | 覆盖 PowerShell 下载/解压超时；默认 `120000`。 |

## 部署清单

1. 添加一个且仅一个与运行时匹配的 Java 依赖。
2. 将 `tikeo.worker.endpoint` 设置为 Worker 能访问到的 Worker Tunnel endpoint。
3. 设置 `namespace`、`app`、`cluster`、`region`，使其符合路由和运维模型。
4. 添加调度器用于 worker selection 的 capabilities 和 labels。
5. 需要稳定生成 instance identity 时持久化 `tikeo.worker.state-dir`。
6. 不可变或受限生产镜像中，预安装/缓存 sandbox tools，并将 `auto-install` / `auto-install-tools` 设为 `false`。
7. 如果启用 management client，显式设置 `tikeo.management.endpoint`，并从 Secret store 注入 `api-key`。
8. 确认 Web 控制台能看到 worker，然后触发路由到 Java capability 的任务并检查日志/结果。

## 本地验证命令

```bash
cd sdks/java
./gradlew test --no-daemon
./gradlew jar sourcesJar --no-daemon
```

```bash
cd examples/java/spring-boot2-worker-demo && ./gradlew test --no-daemon
cd examples/java/spring-boot3-worker-demo && ./gradlew test --no-daemon
cd examples/java/spring-boot4-worker-demo && ./gradlew test --no-daemon
```

## 兼容性规则

Java 模块必须保留明确的 source/resource/test 边界。不要用空模块或 source-set indirection 取代兼容模块。Boot 2、Boot 3、Boot 4 starter 分开存在，是为了让每条兼容线都有真实源码、资源、测试和依赖边界。
