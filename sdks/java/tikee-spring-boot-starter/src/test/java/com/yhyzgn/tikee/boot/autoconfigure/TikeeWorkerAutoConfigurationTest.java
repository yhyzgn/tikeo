package com.yhyzgn.tikee.boot.autoconfigure;

import static org.assertj.core.api.Assertions.assertThat;

import com.yhyzgn.tikee.boot.lifecycle.TikeeWorkerLifecycle;
import com.yhyzgn.tikee.management.client.TikeeJobClient;
import com.yhyzgn.tikee.processor.TikeeProcessor;
import com.yhyzgn.tikee.spring.processor.TikeeProcessorRegistry;
import com.yhyzgn.tikee.worker.client.NoopTikeeWorkerClient;
import com.yhyzgn.tikee.worker.client.TikeeWorkerClient;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.springframework.boot.test.context.runner.ApplicationContextRunner;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

class TikeeWorkerAutoConfigurationTest {
    @TempDir
    Path stateDir;

    private final ApplicationContextRunner contextRunner = new ApplicationContextRunner()
            .withUserConfiguration(TikeeWorkerAutoConfiguration.class, ProcessorConfig.class)
            .withPropertyValues(
                    "tikee.worker.dry-run=true",
                    "tikee.worker.app=billing",
                    "tikee.worker.wasm.auto-install=false");

    @Test
    void dryRunCreatesNoopClientWithGeneratedRegistrationHint() throws Exception {
        installFakeWasmtime(stateDir);
        contextRunner.withPropertyValues("tikee.worker.state-dir=" + stateDir).run(context -> {
            assertThat(context).hasSingleBean(TikeeWorkerClient.class);
            TikeeWorkerClient client = context.getBean(TikeeWorkerClient.class);
            assertThat(client).isInstanceOf(NoopTikeeWorkerClient.class);
            NoopTikeeWorkerClient noop = (NoopTikeeWorkerClient) client;
            assertThat(noop.registration().clientInstanceId()).startsWith("java-");
            assertThat(noop.registration().app()).isEqualTo("billing");
            assertThat(noop.registration().capabilities()).contains("script:shell");
            assertThat(noop.running()).isTrue();
            assertThat(context.getBean(TikeeProcessorRegistry.class).handlers()).containsKey("demo.echo");
        });
    }

    @Test
    void explicitClientInstanceIdOverridesGeneratedValue() {
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.client-instance-id=test-instance").run(context -> {
            NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
            assertThat(noop.registration().clientInstanceId()).isEqualTo("test-instance");
        });
    }


    @Test
    void wasmSandboxAdvertisesScriptWasmWhenRuntimeCheckIsDisabled() throws Exception {
        installFakeWasmtime(stateDir);
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.wasm.auto-install=false")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(noop.registration().capabilities()).contains("script:wasm");
                });
    }

    @Test
    void disablingScriptsDisablesDefaultWasmSandboxCapability() {
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.scripts.enabled=false",
                "tikee.worker.wasm.auto-install=false")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(noop.registration().capabilities()).doesNotContain("script:wasm");
                });
    }

    @Test
    void wasmSandboxIsNotAdvertisedWhenConfiguredRuntimeIsUnavailable() {
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.wasm.auto-install=false",
                "tikee.worker.wasm.install-dir=" + stateDir.resolve("missing-wasmtime"))
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(noop.registration().capabilities()).doesNotContain("script:wasm", "script:shell");
                });
    }

    @Test
    void enablingScriptsDefaultsToWasmShellSandboxWithoutContainerRuntime() throws Exception {
        installFakeWasmtime(stateDir);
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.scripts.enabled=true",
                "tikee.worker.wasm.auto-install=false")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(noop.registration().capabilities())
                            .contains("script:wasm", "script:shell")
                            .doesNotContain("script:python", "script:js", "script:ts", "script:powershell");
                });
    }

    @Test
    void enablingSandboxScriptsAdvertisesScriptCapabilitiesWhenRuntimeCheckIsDisabled() {
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.scripts.enabled=true",
                "tikee.worker.scripts.container-enabled=true",
                "tikee.worker.scripts.availability-check=false",
                "tikee.worker.scripts.runtime-command=test-container-runtime",
                "tikee.worker.scripts.images.shell=alpine:3.20",
                "tikee.worker.scripts.images.python=python:3.13-alpine",
                "tikee.worker.scripts.images.js=denoland/deno:alpine",
                "tikee.worker.scripts.images.ts=denoland/deno:alpine",
                "tikee.worker.scripts.images.powershell=mcr.microsoft.com/powershell:7.5-alpine-3.20")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(noop.registration().capabilities())
                            .contains("script:shell", "script:python", "script:js", "script:ts", "script:powershell");
                });
    }

    @Test
    void sandboxScriptsUseWasmShellWhenContainerRuntimeCommandIsMissing() throws Exception {
        installFakeWasmtime(stateDir);
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.scripts.enabled=true",
                "tikee.worker.scripts.container-enabled=true",
                "tikee.worker.scripts.availability-check=false",
                "tikee.worker.scripts.images.shell=alpine:3.20")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(noop.registration().capabilities())
                            .contains("script:shell")
                            .doesNotContain("script:python", "script:js", "script:ts", "script:powershell");
                });
    }

    @Test
    void unavailableContainerRuntimeKeepsDefaultWasmShellOnly() throws Exception {
        installFakeWasmtime(stateDir);
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.scripts.enabled=true",
                "tikee.worker.scripts.container-enabled=true",
                "tikee.worker.scripts.availability-check=true",
                "tikee.worker.scripts.runtime-command=tikee-missing-container-runtime")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(noop.registration().capabilities())
                            .contains("script:shell")
                            .doesNotContain("script:python", "script:js", "script:ts", "script:powershell");
                });
    }

    @Test
    void managementClientIsConditionalOnManagementFlag() {
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.management.enabled=true",
                "tikee.management.endpoint=http://127.0.0.1:19999",
                "tikee.management.token=test-token",
                "tikee.management.namespace=demo-ns",
                "tikee.management.app=demo-app").run(context -> {
            assertThat(context).hasSingleBean(TikeeJobClient.class);
        });
    }

    @Test
    void managementClientIsDisabledByDefault() {
        contextRunner.withPropertyValues("tikee.worker.state-dir=" + stateDir).run(context -> {
            assertThat(context).doesNotHaveBean(TikeeJobClient.class);
        });
    }

    @Test
    void autoStartupCanBeDisabledWhileKeepingClientBean() {
        contextRunner
                .withPropertyValues("tikee.worker.state-dir=" + stateDir, "tikee.worker.auto-startup=false")
                .run(context -> {
                    assertThat(context).hasSingleBean(TikeeWorkerClient.class);
                    assertThat(context).hasSingleBean(TikeeWorkerLifecycle.class);
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(noop.running()).isFalse();
                });
    }

    @Test
    void disabledWorkerDoesNotCreateClientOrLifecycle() {
        contextRunner
                .withPropertyValues("tikee.worker.enabled=false")
                .run(context -> {
                    assertThat(context).doesNotHaveBean(TikeeWorkerClient.class);
                    assertThat(context).doesNotHaveBean(TikeeWorkerLifecycle.class);
                    assertThat(context).hasSingleBean(TikeeProcessorRegistry.class);
                });
    }

    private static void installFakeWasmtime(Path stateDir) throws Exception {
        Path binary = stateDir.resolve("wasmtime").resolve("bin").resolve("wasmtime");
        java.nio.file.Files.createDirectories(binary.getParent());
        java.nio.file.Files.writeString(binary, "#!/usr/bin/env sh\necho wasmtime 0.0.0-test\n");
        binary.toFile().setExecutable(true);
    }

    @Configuration(proxyBeanMethods = false)
    static class ProcessorConfig {
        @Bean
        DemoProcessor demoProcessor() {
            return new DemoProcessor();
        }
    }

    static class DemoProcessor {
        @TikeeProcessor("demo.echo")
        public String echo(String payload) {
            return payload;
        }
    }
}
