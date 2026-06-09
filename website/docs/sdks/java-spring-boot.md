---
title: Java Spring Boot Starter
description: Java SDK artifacts, dependency selection, Spring Boot configuration, environment variables, and deployment checklist.
---

# Java Spring Boot Starter

The Java SDK is published as Maven Central artifacts under group `net.tikeo`. A service should add **one** Tikeo dependency: either the plain Java SDK, the matching Spring Boot starter, or one advanced Spring Framework adapter. Do not explicitly add the upstream/transitive Tikeo modules that the selected dependency already brings in.

## Runtime and version placeholder

- Java runtime: Java 17+.
- Repository CI validates the SDK on Temurin 21.
- Replace `<TIKEO_VERSION>` with the version shown by the README package badge for the artifact you are installing.
- Go-style `v<TIKEO_VERSION>` tags are not used for Maven Central; Java dependencies use `<TIKEO_VERSION>` without a leading `v`.

## Pick exactly one Java artifact

| Artifact | Use it when... | Dependency line |
| --- | --- | --- |
| `net.tikeo:tikeo` | Plain Java workers, management clients, sandbox tooling, or low-level Worker Tunnel integration. | `implementation("net.tikeo:tikeo:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring-boot-starter` | Spring Boot 4 / Spring Framework 7 app with auto-configuration. | `implementation("net.tikeo:tikeo-spring-boot-starter:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring-boot3-starter` | Spring Boot 3 / Spring Framework 6 app with auto-configuration. | `implementation("net.tikeo:tikeo-spring-boot3-starter:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring-boot2-starter` | Spring Boot 2 / Spring Framework 5 app with auto-configuration. | `implementation("net.tikeo:tikeo-spring-boot2-starter:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring` | Advanced manual Spring Framework 7 wiring without Boot auto-configuration. | `implementation("net.tikeo:tikeo-spring:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring6` | Advanced manual Spring Framework 6 wiring without Boot auto-configuration. | `implementation("net.tikeo:tikeo-spring6:<TIKEO_VERSION>")` |
| `net.tikeo:tikeo-spring5` | Advanced manual Spring Framework 5 wiring without Boot auto-configuration. | `implementation("net.tikeo:tikeo-spring5:<TIKEO_VERSION>")` |

The Spring Boot starters transitively include the matching Spring adapter and core SDK. For example, a Spring Boot 3 service should depend on `tikeo-spring-boot3-starter` only; it should not also declare `tikeo-spring6` or `tikeo` unless you are deliberately overriding dependency resolution.

## Gradle Kotlin DSL

```kotlin
repositories {
    mavenCentral()
}

dependencies {
    // Pick exactly one:
    implementation("net.tikeo:tikeo:<TIKEO_VERSION>")                    // plain Java
    // implementation("net.tikeo:tikeo-spring-boot-starter:<TIKEO_VERSION>")  // Spring Boot 4
    // implementation("net.tikeo:tikeo-spring-boot3-starter:<TIKEO_VERSION>") // Spring Boot 3
    // implementation("net.tikeo:tikeo-spring-boot2-starter:<TIKEO_VERSION>") // Spring Boot 2

    // Advanced adapters, also pick only one when needed:
    // implementation("net.tikeo:tikeo-spring:<TIKEO_VERSION>")  // Spring Framework 7
    // implementation("net.tikeo:tikeo-spring6:<TIKEO_VERSION>") // Spring Framework 6
    // implementation("net.tikeo:tikeo-spring5:<TIKEO_VERSION>") // Spring Framework 5
}
```

## Maven

Use one dependency block and substitute the `artifactId` from the table above:

```xml
<dependency>
  <groupId>net.tikeo</groupId>
  <artifactId>tikeo-spring-boot3-starter</artifactId>
  <version>&lt;TIKEO_VERSION&gt;</version>
</dependency>
```

Common `artifactId` values:

- `tikeo` — plain Java.
- `tikeo-spring-boot-starter` — Spring Boot 4.
- `tikeo-spring-boot3-starter` — Spring Boot 3.
- `tikeo-spring-boot2-starter` — Spring Boot 2.
- `tikeo-spring`, `tikeo-spring6`, `tikeo-spring5` — advanced manual Spring Framework adapters.

## Minimal Spring Boot worker configuration

Set `endpoint` to the Worker Tunnel address reachable from the worker process. Local demos use `127.0.0.1`; Kubernetes/VM deployments usually use a Service, load balancer, or private DNS name.

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

## Complete Spring Boot configuration template

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

## Worker properties and defaults

| Property | Default | What to set in production |
| --- | --- | --- |
| `tikeo.worker.enabled` | `true` | Keep enabled for worker services; disable only for profiles that should not register workers. |
| `tikeo.worker.auto-startup` | `true` | Keep enabled unless you manually control lifecycle. |
| `tikeo.worker.endpoint` | `http://0.0.0.0:9998` | Set explicitly to the Worker Tunnel endpoint reachable from this service. |
| `tikeo.worker.dry-run` | `false` | Keep `false` for live workers; use `true` for config-only smoke tests. |
| `tikeo.worker.heartbeat-interval-millis` | `10000` | Increase only for high-latency networks after validating failure detection behavior. |
| `tikeo.worker.client-instance-id` | blank | Optional. Prefer blank plus persisted `state-dir` for replica-safe generated identities. |
| `tikeo.worker.state-dir` | blank → `~/.tikeo/workers` | Persist this path if you need stable generated instance ids across restarts. |
| `tikeo.worker.namespace` | `default` | Set to your tenant/environment namespace. |
| `tikeo.worker.app` | `default` | Set to the app boundary used by routing and management scopes. |
| `tikeo.worker.cluster` | `default` | Set to cluster, environment, or pool name. |
| `tikeo.worker.region` | `default` | Set to region/zone for routing and operations. |
| `tikeo.worker.capabilities` | `[]` | Add routing capabilities such as `java`, `spring-boot`, `billing`, or `reports`. |
| `tikeo.worker.labels` | `{}` | Add operational labels such as `worker_pool`, `runtime`, `team`, and `tier`. |
| `tikeo.worker.election.enabled` | `true` | Keep enabled for worker-cluster master election unless the service should never lead. |
| `tikeo.worker.election.domain` | blank | Blank means `namespace/app/cluster/region`; set when multiple logical pools share those values. |
| `tikeo.worker.election.priority` | `100` | Lower values win. Use fixed values for deterministic leadership preference. |

## Sandbox and script-tool defaults

| Property | Default | Notes |
| --- | --- | --- |
| `tikeo.worker.wasm.auto-install` | `true` | Installs Wasmtime automatically when missing. Disable in immutable production images. |
| `tikeo.worker.wasm.install-version` | `latest` | Wasmtime installer version, for example `latest` or `v45.0.0`. |
| `tikeo.worker.wasm.install-dir` | blank → `~/.tikeo/sandbox-tools/wasmtime` | Persist/cache this directory to avoid repeated downloads. |
| `tikeo.worker.wasm.installer-url` | `https://wasmtime.dev/install.sh` | Override for internal mirrors. |
| `tikeo.worker.wasm.install-timeout-millis` | `120000` | Installer timeout. |
| `tikeo.worker.scripts.enabled` | `true` | Enables dynamic script execution through the default sandbox path. |
| `tikeo.worker.scripts.container-enabled` | `false` | Enables optional Docker/Podman-backed language runners. |
| `tikeo.worker.scripts.availability-check` | `true` | Probes tools before advertising capabilities. |
| `tikeo.worker.scripts.runtime-command` | blank | Set to `docker`, `podman`, or another compatible runtime when container scripts are enabled. |
| `tikeo.worker.scripts.runtime-args` | `[]` | Extra runtime args appended before the image. |
| `tikeo.worker.scripts.auto-install-tools` | `true` | Auto-installs local development tools when absent. Disable for locked-down production hosts. |
| `tikeo.worker.scripts.srt-install-version` | `latest` | Anthropic Sandbox Runtime npm package version. |
| `tikeo.worker.scripts.srt-install-dir` | blank → `~/.tikeo/sandbox-tools/srt` | SRT install/cache directory. |
| `tikeo.worker.scripts.ripgrep-install-version` | `latest` | ripgrep version required by SRT. |
| `tikeo.worker.scripts.ripgrep-install-dir` | blank → `~/.tikeo/sandbox-tools/ripgrep` | ripgrep install/cache directory. |
| `tikeo.worker.scripts.deno-install-version` | `latest` | Deno installer version. |
| `tikeo.worker.scripts.deno-install-dir` | blank → `~/.tikeo/sandbox-tools/deno` | Deno install/cache directory. |
| `tikeo.worker.scripts.deno-installer-url` | `https://deno.land/install.sh` | Override for internal mirrors. |
| `tikeo.worker.scripts.rhai-install-version` | blank | Blank means latest cargo-installable Rhai tooling. |
| `tikeo.worker.scripts.rhai-install-dir` | blank → `~/.tikeo/sandbox-tools/rhai` | Rhai install/cache directory. |
| `tikeo.worker.scripts.power-shell-install-version` | `7.5.4` | PowerShell Core version for SRT-backed PowerShell. |
| `tikeo.worker.scripts.power-shell-install-dir` | blank → `~/.tikeo/sandbox-tools/pwsh` | PowerShell install/cache directory. Persist/cache it to avoid repeated archive downloads. |
| `tikeo.worker.scripts.wasmedge-auto-install` | `false` | Disabled until explicitly selected. |
| `tikeo.worker.scripts.wasmedge-install-version` | `latest` | WasmEdge installer version. |
| `tikeo.worker.scripts.wasmedge-install-dir` | blank → `~/.tikeo/sandbox-tools/wasmedge` | WasmEdge install/cache directory. |
| `tikeo.worker.scripts.wasmedge-installer-url` | `https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh` | Override for internal mirrors. |
| `tikeo.worker.scripts.v8-install-version` | `latest` | V8 runtime version. |
| `tikeo.worker.scripts.v8-install-dir` | blank → `~/.tikeo/sandbox-tools/v8` | V8 install/cache directory. |
| `tikeo.worker.scripts.tool-install-timeout-millis` | `120000` | Tool installer timeout. |
| `tikeo.worker.scripts.images.shell` | blank | Container image for shell scripts; blank disables that image. |
| `tikeo.worker.scripts.images.python` | blank | Container image for Python scripts; blank disables that image. |
| `tikeo.worker.scripts.images.js` | blank | Container image for JavaScript scripts; blank disables that image. |
| `tikeo.worker.scripts.images.ts` | blank | Container image for TypeScript scripts; blank disables that image. |
| `tikeo.worker.scripts.images.powershell` | blank | Container image for PowerShell scripts; blank disables that image. |
| `tikeo.worker.scripts.images.php` | blank | Container image for PHP scripts; blank disables that image. |
| `tikeo.worker.scripts.images.groovy` | blank | Container image for Groovy scripts; blank disables that image. |
| `tikeo.worker.scripts.images.rhai` | blank | Container image for Rhai scripts; blank disables that image. |

## Management client properties

| Property | Default | What to set in production |
| --- | --- | --- |
| `tikeo.management.enabled` | `false` | Enable only in services that need management/control-plane SDK clients. |
| `tikeo.management.endpoint` | `http://127.0.0.1:9999` | Set explicitly. Compose examples usually expose server HTTP on `9090`. |
| `tikeo.management.api-key` | blank | Put the app-scoped API key in a Secret store and inject it as an environment variable. |
| `tikeo.management.namespace` | `default` | Scope management operations to the intended namespace. |
| `tikeo.management.app` | `default` | Scope management operations to the intended app. |

## Environment variables used by examples

The Spring Boot demos map environment variables into `application.yml`. You can keep the same names in production or map your platform-specific configuration names to the Spring properties above.

| Env var | Maps to | Default in examples |
| --- | --- | --- |
| `TIKEO_DEMO_SERVER_PORT` | `server.port` | `18082`, `18083`, or `18084` depending on demo. |
| `TIKEO_WORKER_DRY_RUN` | `tikeo.worker.dry-run` | `false` |
| `TIKEO_WORKER_ENDPOINT` | `tikeo.worker.endpoint` | `http://127.0.0.1:9998` |
| `TIKEO_WORKER_CLIENT_INSTANCE_ID` | `tikeo.worker.client-instance-id` | demo-specific value using `${HOSTNAME}` fallback. |
| `TIKEO_WORKER_STATE_DIR` | `tikeo.worker.state-dir` | blank |
| `TIKEO_WORKER_NAMESPACE` | `tikeo.worker.namespace` | `default` or `dev-alpha` depending on demo. |
| `TIKEO_WORKER_APP` | `tikeo.worker.app` | `default`, `orders`, or `billing` depending on demo. |
| `TIKEO_WORKER_CLUSTER` | `tikeo.worker.cluster` | `local` |
| `TIKEO_WORKER_REGION` | `tikeo.worker.region` | `local` |
| `TIKEO_WORKER_POOL` | `tikeo.worker.labels.worker_pool` | `boot2-blue`, `boot3-blue`, or `boot4-green` |
| `TIKEO_WORKER_WASM_AUTO_INSTALL` | `tikeo.worker.wasm.auto-install` | `true` |
| `TIKEO_WORKER_WASM_VERSION` | `tikeo.worker.wasm.install-version` | `latest` |
| `TIKEO_WORKER_WASM_INSTALL_DIR` | `tikeo.worker.wasm.install-dir` | blank |
| `TIKEO_WORKER_SCRIPTS_ENABLED` | `tikeo.worker.scripts.enabled` | `true` |
| `TIKEO_WORKER_CONTAINER_SCRIPTS_ENABLED` | `tikeo.worker.scripts.container-enabled` | `false` |
| `TIKEO_WORKER_CONTAINER_RUNTIME` | `tikeo.worker.scripts.runtime-command` | blank |
| `TIKEO_WORKER_SCRIPT_RUNTIME_CHECK` | `tikeo.worker.scripts.availability-check` | `true` |
| `TIKEO_WORKER_SCRIPT_AUTO_INSTALL_TOOLS` | `tikeo.worker.scripts.auto-install-tools` | `true` |
| `TIKEO_WORKER_SCRIPT_SHELL_IMAGE` | `tikeo.worker.scripts.images.shell` | blank |
| `TIKEO_WORKER_SCRIPT_PYTHON_IMAGE` | `tikeo.worker.scripts.images.python` | blank |
| `TIKEO_WORKER_SCRIPT_JAVASCRIPT_IMAGE` | `tikeo.worker.scripts.images.js` | blank |
| `TIKEO_WORKER_SCRIPT_TYPESCRIPT_IMAGE` | `tikeo.worker.scripts.images.ts` | blank |
| `TIKEO_WORKER_SCRIPT_POWERSHELL_IMAGE` | `tikeo.worker.scripts.images.powershell` | blank |
| `TIKEO_WORKER_SCRIPT_RHAI_IMAGE` | `tikeo.worker.scripts.images.rhai` | blank |
| `TIKEO_MANAGEMENT_ENABLED` | `tikeo.management.enabled` | `true` in demos, `false` in starter source defaults. |
| `TIKEO_MANAGEMENT_ENDPOINT` | `tikeo.management.endpoint` | `http://127.0.0.1:9999` |
| `TIKEO_MANAGEMENT_API_KEY` | `tikeo.management.api-key` | Demo token only; replace in real deployments. |
| `TIKEO_MANAGEMENT_NAMESPACE` | `tikeo.management.namespace` | `dev-alpha` |
| `TIKEO_MANAGEMENT_APP` | `tikeo.management.app` | `orders` or `billing` |

Low-level Java SDK identity and installer environment variables:

| Env var | Meaning |
| --- | --- |
| `TIKEO_WORKER_RUNTIME_ID` | Runtime identity input used when generating a client instance id. |
| `TIKEO_POD_NAME`, `POD_NAME`, `HOSTNAME` | Fallback runtime identity inputs for generated worker instance ids. |
| `TIKEO_POWERSHELL_VERSION` | Fallback PowerShell installer version; default `7.5.4`. |
| `TIKEO_POWERSHELL_DOWNLOAD_URL` | Override the PowerShell archive URL for mirrored/offline environments. |
| `TIKEO_POWERSHELL_INSTALL_TIMEOUT_MILLIS` | Override PowerShell download/extract timeout; default `120000`. |

## Deployment checklist

1. Add exactly one Java dependency that matches your runtime.
2. Set `tikeo.worker.endpoint` to the Worker Tunnel endpoint reachable from the worker.
3. Set `namespace`, `app`, `cluster`, and `region` to values that match your routing and operations model.
4. Add capabilities and labels that the scheduler can use for worker selection.
5. Persist `tikeo.worker.state-dir` when stable generated instance identity matters.
6. In immutable or restricted production images, preinstall/cache sandbox tools and set `auto-install` / `auto-install-tools` to `false`.
7. If management clients are enabled, set `tikeo.management.endpoint` explicitly and inject `api-key` from a Secret store.
8. Verify that the worker appears in the Web console, then trigger a task routed to a Java capability and confirm logs/results.

## Local verification commands

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

## Compatibility rule

Java modules must keep explicit source/resource/test boundaries. Do not replace compatibility modules with empty source-set indirection. The separate Boot 2, Boot 3, and Boot 4 starters exist so each compatibility line has real source, resources, tests, and dependency boundaries.
