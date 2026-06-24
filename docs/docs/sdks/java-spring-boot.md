---
title: Java SDK and Spring Boot Starter
description: Java SDK module coordinates, Spring Boot property defaults, @TikeoProcessor Worker setup, Management API helpers, and live verification runbook.
---

# Java SDK and Spring Boot Starter

The Java SDK lives in `sdks/java` as a Gradle multi-module build. It supports a core Worker/Management SDK, Spring Framework adapters, and Spring Boot starters for Boot 2, Boot 3, and Boot 4 compatibility lines. Runnable demos live under `examples/java/spring-boot{2,3,4}-worker-demo`.


Shared SDK/API contract: see [SDK and API integration guide](../integrations/sdk-and-api) for common concepts, unified configuration parameters, Management API semantics, Worker connection parameters, trigger types, errors/retries, and the language difference table. This language page stays focused on installation, minimal Worker code, exception behavior, and Management client syntax.

## Dependency coordinates

`sdks/java/settings.gradle.kts` declares these modules:

| Module | Artifact intent |
| --- | --- |
| `tikeo` | Core Java SDK: gRPC Worker client, task models, Management client. |
| `tikeo-spring` | Spring Framework 7 processor registry/adapter. |
| `tikeo-spring6` | Spring Framework 6 adapter for Boot 3. |
| `tikeo-spring5` | Spring Framework 5 adapter for Boot 2. |
| `tikeo-spring-boot-starter` | Spring Boot 4 starter. |
| `tikeo-spring-boot3-starter` | Spring Boot 3 starter. |
| `tikeo-spring-boot2-starter` | Spring Boot 2 starter. |

The group is `net.tikeo`; replace `${TIKEO_VERSION}` with the version shown by the README/top package badge or the release tag. In this repository the editable development version lives in `sdks/java/gradle.properties` as `tikeoVersion`; release workflows synchronize it from the tag. Java release is `17`, and current dependency baselines include gRPC `1.81.0`, protobuf `4.34.1`, Jackson `2.20.1`, Spring Framework 5/6/7, and Spring Boot 2/3/4 version properties.

Gradle dependency examples:

```kotlin
dependencies {
    implementation("net.tikeo:tikeo-spring-boot3-starter:${TIKEO_VERSION}")
}
```

```xml
<dependency>
  <groupId>net.tikeo</groupId>
  <artifactId>tikeo-spring-boot3-starter</artifactId>
  <version>${tikeo.version}</version>
</dependency>
```

Verify locally:

```bash
./sdks/java/gradlew -p sdks/java test --no-daemon
./sdks/java/gradlew -p sdks/java :tikeo:test --no-daemon
```

## Spring Boot property defaults

`sdks/java/tikeo-spring-boot3-starter/src/main/java/net/tikeo/boot/autoconfigure/TikeoWorkerProperties.java` is representative for Boot starter defaults.

| Property | Default | Meaning |
| --- | --- | --- |
| `tikeo.worker.enabled` | `true` | Enable Worker auto-configuration. |
| `tikeo.worker.auto-startup` | `true` | Start with Spring lifecycle. Startup is non-fatal when the Tikeo Server / Worker Tunnel is temporarily unreachable: the starter logs a warning, the business application continues booting, and the worker client retries in the background. |
| `tikeo.worker.endpoint` | `http://127.0.0.1:9998` | Local Worker Tunnel endpoint; override to the reachable Service/LB/Gateway URL in deployments. |
| `tikeo.worker.dry-run` | `false` | Avoid live tunnel when true. |
| `tikeo.worker.heartbeat-interval-millis` | `10000` | Lease renewal cadence. |
| `tikeo.worker.client-instance-id` | blank | If blank, SDK generates and persists one per scope/runtime identity. |
| `tikeo.worker.state-dir` | blank | Blank means `~/.tikeo/workers`. |
| `tikeo.worker.namespace` | `default` | Demo overrides to `dev-alpha`. |
| `tikeo.worker.app` | `default` | Demo overrides to `orders`. |
| `tikeo.worker.cluster` | `default` | Demo overrides to `local`. |
| `tikeo.worker.region` | `default` | Demo overrides to `local`. |
| `tikeo.worker.capabilities` | `[]` | Also used as tags by auto-config. |
| `tikeo.worker.labels` | `{}` | Demo adds `worker_pool`, `runtime`, `demo`. |
| `tikeo.worker.election.enabled` | `true` | Worker-cluster election metadata. |
| `tikeo.worker.election.domain` | blank | Blank means namespace/app/cluster/region. |
| `tikeo.worker.election.priority` | `100` | Lower wins. |
| `tikeo.worker.scripts.enabled` | `true` | Dynamic script registry enabled. |
| `tikeo.worker.scripts.container-enabled` | `false` | Container-backed non-WASM scripts disabled by default. |
| `tikeo.worker.scripts.auto-install-tools` | `true` | Background local development tool prewarm; startup never waits for downloads. |
| `tikeo.worker.scripts.strict-sandbox-isolation` | `false` | Strict sandbox isolation switch: ignore host PATH tools/interpreters and use only sandbox-tools cache binaries. Env: `TIKEO_WORKER_SCRIPTS_STRICT_SANDBOX_ISOLATION`. |
| `tikeo.management.enabled` | `false` in starter, demos set true | Auto-configure `TikeoJobClient`. |
| `tikeo.management.endpoint` | `http://127.0.0.1:9090` | HTTP Management endpoint; override to your Server API URL. |
| `tikeo.management.api-key` | blank | App-scoped SDK API key. |
| `tikeo.management.namespace` | `default` | Management scope. |
| `tikeo.management.app` | `default` | Management scope. |

Spring Boot relaxed binding means environment variables such as `TIKEO_WORKER_ENDPOINT`, `TIKEO_WORKER_NAMESPACE`, and `TIKEO_MANAGEMENT_API_KEY` work in the demos' `application.yml`.

## Minimal Worker

```java
package com.example.worker;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TikeoProcessor;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

@Component
public final class EchoProcessor {
    private static final Logger log = LoggerFactory.getLogger(EchoProcessor.class);

    @TikeoProcessor("demo.echo")
    public String echo(TaskContext context, String payload) {
        log.info("java echo processor={} instance={}", context.processorName(), context.instanceId());
        return "java echo processed: " + payload;
    }
}
```

Add `net.tikeo.logging.TikeoTaskLogbackAppender` to Logback (the demos provide `logback-spring.xml`). Ordinary SLF4J records emitted while a processor runs are mirrored to the current job instance via a thread-local task scope and MDC keys; startup and unrelated request logs are not attached. `TaskContext.logInfo/logError` remains a fallback.

`@TikeoProcessor` is scanned by `TikeoProcessorRegistry`, converted into structured normal processor capabilities, and invoked by `SpringTikeoTaskProcessor` when `DispatchTask.processor_name` matches. Do not rely on job ID naming as the primary dispatch contract.

Example `application.yml` for a service:

```yaml
tikeo:
  worker:
    endpoint: ${TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}
    namespace: ${TIKEO_WORKER_NAMESPACE:sdk-smoke}
    app: ${TIKEO_WORKER_APP:management}
    cluster: ${TIKEO_WORKER_CLUSTER:local}
    region: ${TIKEO_WORKER_REGION:local}
    labels:
      worker_pool: ${TIKEO_WORKER_POOL:boot3-blue}
  management:
    enabled: true
    endpoint: ${TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9090}
    api-key: ${TIKEO_MANAGEMENT_API_KEY:}
    namespace: ${TIKEO_MANAGEMENT_NAMESPACE:sdk-smoke}
    app: ${TIKEO_MANAGEMENT_APP:management}
```

## Demo commands

Boot 3 example:

```bash
cd examples/java/spring-boot3-worker-demo
TIKEO_WORKER_DRY_RUN=true ./gradlew bootRun --no-daemon
```

Live example:

```bash
TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998 \
TIKEO_WORKER_NAMESPACE=sdk-smoke \
TIKEO_WORKER_APP=management \
TIKEO_WORKER_POOL=boot3-blue \
TIKEO_MANAGEMENT_ENDPOINT=http://127.0.0.1:9090 \
TIKEO_MANAGEMENT_API_KEY="$TIKEO_MANAGEMENT_API_KEY" \
./gradlew bootRun --no-daemon
```

The demo exposes local HTTP helper endpoints for examples, but that is demo application behavior. It does not change the Tikeo architecture: the Worker still receives tasks through the outbound Worker Tunnel.

## Management API create + trigger

Core Java Management client source is `sdks/java/tikeo/src/main/java/net/tikeo/management/client/HttpTikeoJobClient.java`.

```java
import net.tikeo.management.client.HttpTikeoJobClient;
import net.tikeo.management.model.CreateJobRequest;
import net.tikeo.management.model.JobDefinition;
import net.tikeo.management.model.JobInstance;
import net.tikeo.management.model.TriggerJobRequest;

var client = new HttpTikeoJobClient(
    System.getenv().getOrDefault("TIKEO_MANAGEMENT_ENDPOINT", "http://127.0.0.1:9090"),
    System.getenv("TIKEO_MANAGEMENT_API_KEY"),
    "sdk-smoke",
    "management"
);

JobDefinition created = client.createJob(CreateJobRequest.api("java-echo-api", "demo.echo"));
JobInstance instance = client.triggerJob(created.id(), TriggerJobRequest.api());

if (!"api".equals(instance.triggerType()) || !"single".equals(instance.executionMode())) {
    throw new IllegalStateException("unexpected trigger response");
}
```

Broadcast is explicit:

```java
import java.util.List;
import java.util.Map;
import net.tikeo.management.model.BroadcastSelectorRequest;
import net.tikeo.management.model.TriggerJobRequest;

var selector = new BroadcastSelectorRequest(
    List.of("manual-demo"),
    "local",
    "local",
    Map.of("worker_pool", "boot3-blue")
);
client.triggerJob(created.id(), TriggerJobRequest.broadcastApi(selector));
```

## Management client credentials

All SDK Management clients use app-scoped service credentials. They send the `x-tikeo-api-key` header, normally sourced from `TIKEO_MANAGEMENT_API_KEY`. Do not confuse this key with a human bearer token from `/api/v1/auth/login`, and do not reuse browser sessions or OIDC provider tokens in SDK services.

The common create+trigger default is:

| Field | Default helper behavior |
| --- | --- |
| Job schedule | `scheduleType=api` |
| Job enabled | `true` |
| Retry policy | `enabled=true`, `maxAttempts=3`, `initialDelaySeconds=5`, `backoffMultiplier=2`, `maxDelaySeconds=60` |
| Trigger source | `triggerType=api` |
| Trigger execution mode | `executionMode=single` |
| Broadcast | Opt-in only through explicit broadcast helper and `broadcastSelector` |

## Operator-verified reference links

Keep SDK helper docs anchored to operator-verified API and protocol references:

- Create helper endpoint: [`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- Trigger helper endpoint: [`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- Instance polling endpoint: [`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- Instance log endpoint: [`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker dispatch message: [`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## Live verification runbook

1. Start the Server with `cargo run --bin tikeo -- serve --config config/dev.toml`.
2. Bootstrap an Owner or login to an existing local Owner.
3. Create namespace/app/worker pool, service account, and SDK API key as shown in the quickstart.
4. Start the language demo Worker with matching namespace/app and `TIKEO_WORKER_CONNECT=1` when the demo supports live mode.
5. Create and trigger an API job through the language Management client.
6. Inspect `/api/v1/workers`, `/api/v1/instances`, instance logs, and audit logs.
7. Preserve smoke evidence. For a maintained end-to-end proof, run `TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh`.

Expected acceptance evidence includes an online worker with the requested structured processor, an API-triggered instance with `executionMode=single`, task logs from the Worker, and a successful processor message. Missing sandbox tools or unsupported processors must fail closed and be visible in task/diagnostic logs.

## Failure and exception demos

The Spring Boot demos separate expected business failures from runtime exceptions. `demo.fail` returns a failed `TaskOutcome`; `demo.exception` throws an `IllegalStateException`. The Worker client records runtime exception stack traces as task logs, and ordinary SLF4J/Logback lines emitted before the exception are mirrored through `TikeoTaskLogbackAppender`, so a live dispatched exception is visible in instance logs and in the public notification console link.

## Capability discipline

The dispatch contract uses structured capabilities, not folklore or only string naming conventions. A Worker should advertise normal processors, plugin processors, script runners, labels, and tags only when the runtime can really execute them. Do not advertise SQL, shell, Python, Node.js, WASM, SRT, Deno, Docker, or Podman support just because a package exists; advertise it after the demo or service has resolved the tool and can fail safely.

## Operational notes

For a Spring Boot service, treat `tikeo.worker.*` as application infrastructure configuration, not as per-job business data. Put stable scope values in deployment configuration, keep `client-instance-id` stable when reconnect correlation matters, and use `state-dir` when you want generated IDs to survive restarts. The starter can generate and persist a client instance ID when the property is blank; that is safer than hardcoding the same ID into every pod.

The auto-configuration merges three capability sources: explicit `tikeo.worker.capabilities` tags, `@TikeoProcessor` registry entries, and available script/WASM runner registries. That means a processor method becomes dispatchable only after Spring creates the bean and the registry scans it. If a job remains pending, check the Worker DTO's structured capabilities first; do not assume the annotation was registered merely because the class compiled.

The demo `DemoJobManagementController` is intentionally a local teaching surface. It shows `TikeoJobClient` creating, disabling, enabling, and triggering jobs, but a real production service should normally keep Management operations behind its own authorization boundary or run them from a control-plane service. The Worker itself still receives task execution over the outbound Worker Tunnel; any demo HTTP endpoint is not a scheduler callback endpoint.

## Prerequisites

Use the setup, authentication, and access requirements described in this page before running any command. For local examples, start the Server with `config/dev.toml`, use `127.0.0.1` as the client host, and keep tokens in shell variables rather than pasted into files.

## Verify

After following the page, verify the result with the documented API, UI, build, smoke, or deployment checks. A valid verification includes the command that was run, the route or file that was inspected, and the observed status or artifact.

## Troubleshooting

When a step fails, first capture the exact command, response status, and Server log window. Then check authentication, namespace/app scope, Worker eligibility, storage readiness, and proxy behavior before changing production configuration.

## Production checklist

- [ ] Secrets are referenced through environment or platform secret mechanisms and are not written into examples.
- [ ] Commands have been adapted from local `127.0.0.1` to the real host, TLS, and authentication model.
- [ ] Rollback and evidence collection are documented for the changed surface.
- [ ] Operators can repeat the verification without private shell history or hidden state.
