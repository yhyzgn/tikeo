package com.yhyzgn.tikee.boot.autoconfigure;

import static org.assertj.core.api.Assertions.assertThat;

import com.yhyzgn.tikee.boot.lifecycle.TikeeWorkerLifecycle;
import com.yhyzgn.tikee.management.client.TikeeJobClient;
import com.yhyzgn.tikee.processor.TikeeProcessor;
import com.yhyzgn.tikee.processor.TikeeProcessorKind;
import com.yhyzgn.tikee.spring.processor.TikeeProcessorRegistry;
import com.yhyzgn.tikee.script.ScriptRunnerKind;
import com.yhyzgn.tikee.script.ScriptRunnerRegistry;
import com.yhyzgn.tikee.script.SrtScriptRunner;
import com.yhyzgn.tikee.worker.client.NoopTikeeWorkerClient;
import com.yhyzgn.tikee.worker.client.TikeeWorkerClient;
import java.nio.file.Path;
import java.util.logging.Logger;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.springframework.boot.test.context.runner.ApplicationContextRunner;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

class TikeeWorkerAutoConfigurationTest {
    private static final Logger log = Logger.getLogger(TikeeWorkerAutoConfigurationTest.class.getName());
    @TempDir
    Path stateDir;

    private final ApplicationContextRunner contextRunner = new ApplicationContextRunner()
            .withUserConfiguration(TikeeWorkerAutoConfiguration.class, ProcessorConfig.class)
            .withPropertyValues(
                    "tikee.worker.dry-run=true",
                    "tikee.worker.app=billing",
                    "tikee.worker.wasm.auto-install=false",
                    "tikee.worker.scripts.auto-install-tools=false");

    private static java.util.List<String> scriptLanguages(NoopTikeeWorkerClient noop) {
        return noop.registration().structuredCapabilities().scriptRunners().stream()
                .map(runner -> runner.language())
                .toList();
    }


    @Test
    void starterPublishesBoot2AndBoot3AutoConfigurationMetadata() throws Exception {
        try (var imports = Thread.currentThread().getContextClassLoader().getResourceAsStream(
                "META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports");
             var factories = Thread.currentThread().getContextClassLoader().getResourceAsStream(
                "META-INF/spring.factories")) {
            assertThat(imports).as("Spring Boot 2.7+/3.x auto-configuration imports").isNotNull();
            assertThat(factories).as("Spring Boot 2.x spring.factories auto-configuration entry").isNotNull();
            assertThat(new String(imports.readAllBytes(), java.nio.charset.StandardCharsets.UTF_8))
                    .contains("com.yhyzgn.tikee.boot.autoconfigure.TikeeWorkerAutoConfiguration");
            assertThat(new String(factories.readAllBytes(), java.nio.charset.StandardCharsets.UTF_8))
                    .contains("org.springframework.boot.autoconfigure.EnableAutoConfiguration")
                    .contains("com.yhyzgn.tikee.boot.autoconfigure.TikeeWorkerAutoConfiguration");
        }
    }

    @Test
    void dryRunCreatesNoopClientWithGeneratedRegistrationHint() throws Exception {
        installFakeWasmtime(stateDir);
        installFakeSrt(stateDir);
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.scripts.container-enabled=false").run(context -> {
            assertThat(context).hasSingleBean(TikeeWorkerClient.class);
            TikeeWorkerClient client = context.getBean(TikeeWorkerClient.class);
            assertThat(client).isInstanceOf(NoopTikeeWorkerClient.class);
            NoopTikeeWorkerClient noop = (NoopTikeeWorkerClient) client;
            assertThat(noop.registration().clientInstanceId()).startsWith("java-");
            assertThat(noop.registration().app()).isEqualTo("billing");
            assertThat(scriptLanguages(noop)).contains("wasm", "shell", "python", "javascript", "typescript", "powershell", "php", "groovy", "rhai");
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
                "tikee.worker.wasm.auto-install=false",
                "tikee.worker.scripts.container-enabled=false")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(scriptLanguages(noop)).contains("wasm");
                });
    }

    @Test
    void disablingScriptsDisablesDefaultWasmSandboxCapability() {
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.scripts.enabled=false",
                "tikee.worker.wasm.auto-install=false",
                "tikee.worker.scripts.container-enabled=false")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(scriptLanguages(noop)).doesNotContain("wasm");
                });
    }

    @Test
    void wasmSandboxIsNotAdvertisedWhenConfiguredRuntimeIsUnavailable() {
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.wasm.auto-install=false",
                "tikee.worker.wasm.install-dir=" + stateDir.resolve("missing-wasmtime"),
                "tikee.worker.scripts.enabled=false")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(scriptLanguages(noop)).doesNotContain("wasm");
                });
    }

    @Test
    void enablingScriptsDefaultsToWasmAndSrtShellWithoutContainerRuntime() throws Exception {
        installFakeWasmtime(stateDir);
        installFakeSrt(stateDir);
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.scripts.enabled=true",
                "tikee.worker.wasm.auto-install=false",
                "tikee.worker.scripts.container-enabled=false",
                "tikee.worker.scripts.auto-install-tools=false")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(scriptLanguages(noop)).contains("wasm", "shell", "python", "javascript", "typescript", "powershell", "php", "groovy", "rhai");
                });
    }

    @Test
    void enablingSandboxScriptsAdvertisesScriptCapabilitiesWhenRuntimeCheckIsDisabled() throws Exception {
        installFakeWasmtime(stateDir);
        installFakeSrt(stateDir);
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.scripts.enabled=true",
                "tikee.worker.scripts.container-enabled=true",
                "tikee.worker.scripts.availability-check=false",
                "tikee.worker.scripts.auto-install-tools=false",
                "tikee.worker.scripts.runtime-command=test-container-runtime",
                "tikee.worker.scripts.images.shell=alpine:3.20",
                "tikee.worker.scripts.images.python=python:3.13-alpine",
                "tikee.worker.scripts.images.js=denoland/deno:alpine",
                "tikee.worker.scripts.images.ts=denoland/deno:alpine",
                "tikee.worker.scripts.images.powershell=mcr.microsoft.com/powershell:7.5-alpine-3.20",
                "tikee.worker.scripts.images.php=php:8.4-cli-alpine",
                "tikee.worker.scripts.images.groovy=groovy:4.0-jdk21",
                "tikee.worker.scripts.images.rhai=rhaiscript/rhai:latest")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(scriptLanguages(noop))
                            .contains("wasm", "shell", "python", "javascript", "typescript", "powershell", "php", "groovy", "rhai");
                    ScriptRunnerRegistry registry = context.getBean(ScriptRunnerRegistry.class);
                    assertThat(registry.find(ScriptRunnerKind.SHELL))
                            .hasValueSatisfying(runner -> assertThat(runner).isInstanceOf(SrtScriptRunner.class));
                });
    }

    @Test
    void sandboxScriptsStayWasmAndSrtShellWhenContainerRuntimeCommandIsMissing() throws Exception {
        installFakeWasmtime(stateDir);
        installFakeSrt(stateDir);
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.scripts.enabled=true",
                "tikee.worker.scripts.container-enabled=true",
                "tikee.worker.scripts.availability-check=false",
                "tikee.worker.scripts.auto-install-tools=false",
                "tikee.worker.scripts.runtime-command=",
                "tikee.worker.scripts.images.shell=alpine:3.20")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(scriptLanguages(noop)).contains("wasm", "shell", "python", "javascript", "typescript", "powershell", "php", "groovy", "rhai");
                });
    }

    @Test
    void unavailableContainerRuntimeKeepsDefaultWasmAndSrtShellOnly() throws Exception {
        installFakeWasmtime(stateDir);
        installFakeSrt(stateDir);
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.scripts.enabled=true",
                "tikee.worker.scripts.container-enabled=true",
                "tikee.worker.scripts.availability-check=true",
                "tikee.worker.scripts.auto-install-tools=false",
                "tikee.worker.scripts.runtime-command=tikee-missing-container-runtime")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(scriptLanguages(noop)).contains("wasm", "shell", "python", "javascript", "typescript", "powershell", "php", "groovy", "rhai");
                });
    }

    @Test
    void managementClientIsConditionalOnManagementFlag() {
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.management.enabled=true",
                "tikee.management.endpoint=http://127.0.0.1:19999",
                "tikee.management.api-key=tk-AbCdEfGhIjKlMnOpQrStUvWxYz0123456789AbCdEfGhIjKlMnOpQrStUv",
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
    void pluginProcessorCapabilityIsAdvertisedFromAnnotationMetadata() throws Exception {
        installFakeWasmtime(stateDir);
        log.info(() -> "[java-sdk-plugin-test] verifying worker registration advertises structured plugin processor metadata");

        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.capabilities[0]=java",
                "tikee.worker.capabilities[1]=spring-boot",
                "tikee.worker.labels.plugin.sql=enabled")
                .run(context -> {
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    log.info(() -> "[java-sdk-plugin-test] registration capabilities="
                            + noop.registration().capabilities());
                    log.info(() -> "[java-sdk-plugin-test] structured capabilities="
                            + noop.registration().structuredCapabilities());
                    log.info(() -> "[java-sdk-plugin-test] registration labels="
                            + noop.registration().labels());
                    assertThat(noop.registration().capabilities())
                            .containsExactly("java", "spring-boot");
                    assertThat(noop.registration().structuredCapabilities().pluginProcessors())
                            .anySatisfy(plugin -> {
                                assertThat(plugin.type()).isEqualTo("sql");
                                assertThat(plugin.processorNames()).contains("billing.sql-sync");
                            });
                    assertThat(noop.registration().labels()).containsEntry("plugin.sql", "enabled");
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
        Path binary = stateDir.resolve("sandbox-tools").resolve("wasmtime").resolve("bin").resolve("wasmtime");
        java.nio.file.Files.createDirectories(binary.getParent());
        java.nio.file.Files.writeString(binary, "#!/usr/bin/env sh\necho wasmtime 0.0.0-test\n");
        binary.toFile().setExecutable(true);
    }

    private static void installFakeSrt(Path stateDir) throws Exception {
        Path binary = stateDir.resolve("sandbox-tools").resolve("srt").resolve("bin").resolve("srt");
        java.nio.file.Files.createDirectories(binary.getParent());
        java.nio.file.Files.writeString(binary, "#!/usr/bin/env sh\necho srt 0.0.0-test\n");
        binary.toFile().setExecutable(true);
    }

    @Configuration(proxyBeanMethods = false)
    static class ProcessorConfig {
        @Bean
        DemoProcessor demoProcessor() {
            return new DemoProcessor();
        }

        @Bean
        DemoPluginProcessor demoPluginProcessor() {
            return new DemoPluginProcessor();
        }
    }

    static class DemoProcessor {
        @TikeeProcessor("demo.echo")
        public String echo(String payload) {
            return payload;
        }
    }

    static class DemoPluginProcessor {
        @TikeeProcessor(value = "billing.sql-sync", kind = TikeeProcessorKind.PLUGIN, pluginType = "sql")
        public String sync(String payload) {
            return payload;
        }
    }
}
