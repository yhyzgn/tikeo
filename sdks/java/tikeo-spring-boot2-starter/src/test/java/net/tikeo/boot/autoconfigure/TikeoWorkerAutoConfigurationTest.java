package net.tikeo.boot.autoconfigure;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.logging.Logger;
import net.tikeo.boot.lifecycle.TikeoWorkerLifecycle;
import net.tikeo.management.client.TikeoJobClient;
import net.tikeo.processor.TikeoProcessor;
import net.tikeo.processor.TikeoProcessorKind;
import net.tikeo.script.ScriptRunnerKind;
import net.tikeo.script.ScriptRunnerRegistry;
import net.tikeo.script.SrtScriptRunner;
import net.tikeo.spring.processor.TikeoProcessorRegistry;
import net.tikeo.worker.client.NoopTikeoWorkerClient;
import net.tikeo.worker.client.TikeoWorkerClient;
import org.assertj.core.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.springframework.boot.test.context.runner.ApplicationContextRunner;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

class TikeoWorkerAutoConfigurationTest {
    private static final Logger log = Logger.getLogger(TikeoWorkerAutoConfigurationTest.class.getName());
    @TempDir
    Path stateDir;

    private final ApplicationContextRunner contextRunner = new ApplicationContextRunner()
            .withUserConfiguration(TikeoWorkerAutoConfiguration.class, ProcessorConfig.class)
            .withPropertyValues(
                    "tikeo.worker.dry-run=true",
                    "tikeo.worker.app=billing",
                    "tikeo.worker.wasm.auto-install=false",
                    "tikeo.worker.scripts.auto-install-tools=false",
                    "tikeo.worker.scripts.deno-install-dir=/tmp/tikeo-test-missing-deno");

    private static List<String> scriptLanguages(NoopTikeoWorkerClient noop) {
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
            Assertions.assertThat(imports).as("Spring Boot 2.7+/3.x auto-configuration imports").isNotNull();
            Assertions.assertThat(factories).as("Spring Boot 2.x spring.factories auto-configuration entry").isNotNull();
            Assertions.assertThat(new String(imports.readAllBytes(), StandardCharsets.UTF_8))
                    .contains("net.tikeo.boot.autoconfigure.TikeoWorkerAutoConfiguration");
            Assertions.assertThat(new String(factories.readAllBytes(), StandardCharsets.UTF_8))
                    .contains("org.springframework.boot.autoconfigure.EnableAutoConfiguration")
                    .contains("net.tikeo.boot.autoconfigure.TikeoWorkerAutoConfiguration");
        }
    }

    @Test
    void dryRunCreatesNoopClientWithGeneratedRegistrationHint() throws Exception {
        installFakeWasmtime(stateDir);
        installFakeSrt(stateDir);
        installFakeRipgrep(stateDir);
        installFakePowerShell(stateDir);
        installFakeRhai(stateDir);
        contextRunner.withPropertyValues(
                "tikeo.worker.state-dir=" + stateDir,
                "tikeo.worker.scripts.container-enabled=false").run(context -> {
            Assertions.assertThat(context).hasSingleBean(TikeoWorkerClient.class);
            TikeoWorkerClient client = context.getBean(TikeoWorkerClient.class);
            Assertions.assertThat(client).isInstanceOf(NoopTikeoWorkerClient.class);
            NoopTikeoWorkerClient noop = (NoopTikeoWorkerClient) client;
            Assertions.assertThat(noop.registration().clientInstanceId()).startsWith("java-");
            Assertions.assertThat(noop.registration().app()).isEqualTo("billing");
            Assertions.assertThat(scriptLanguages(noop)).contains("wasm", "shell");
            Assertions.assertThat(scriptLanguages(noop)).doesNotContain("javascript", "typescript");
            Assertions.assertThat(noop.running()).isTrue();
            Assertions.assertThat(context.getBean(TikeoProcessorRegistry.class).handlers()).containsKey("demo.echo");
        });
    }

    @Test
    void explicitClientInstanceIdOverridesGeneratedValue() {
        contextRunner.withPropertyValues(
                "tikeo.worker.state-dir=" + stateDir,
                "tikeo.worker.client-instance-id=test-instance").run(context -> {
            NoopTikeoWorkerClient noop = context.getBean(NoopTikeoWorkerClient.class);
            Assertions.assertThat(noop.registration().clientInstanceId()).isEqualTo("test-instance");
        });
    }


    @Test
    void wasmSandboxAdvertisesScriptWasmWhenRuntimeCheckIsDisabled() throws Exception {
        installFakeWasmtime(stateDir);
        contextRunner.withPropertyValues(
                "tikeo.worker.state-dir=" + stateDir,
                "tikeo.worker.wasm.auto-install=false",
                "tikeo.worker.scripts.container-enabled=false")
                .run(context -> {
                    NoopTikeoWorkerClient noop = context.getBean(NoopTikeoWorkerClient.class);
                    Assertions.assertThat(scriptLanguages(noop)).contains("wasm");
                });
    }

    @Test
    void disablingScriptsDisablesDefaultWasmSandboxCapability() {
        contextRunner.withPropertyValues(
                "tikeo.worker.state-dir=" + stateDir,
                "tikeo.worker.scripts.enabled=false",
                "tikeo.worker.wasm.auto-install=false",
                "tikeo.worker.scripts.container-enabled=false")
                .run(context -> {
                    NoopTikeoWorkerClient noop = context.getBean(NoopTikeoWorkerClient.class);
                    Assertions.assertThat(scriptLanguages(noop)).doesNotContain("wasm");
                });
    }

    @Test
    void wasmSandboxIsNotAdvertisedWhenConfiguredRuntimeIsUnavailable() {
        contextRunner.withPropertyValues(
                "tikeo.worker.state-dir=" + stateDir,
                "tikeo.worker.wasm.auto-install=false",
                "tikeo.worker.wasm.install-dir=" + stateDir.resolve("missing-wasmtime"),
                "tikeo.worker.scripts.enabled=false")
                .run(context -> {
                    NoopTikeoWorkerClient noop = context.getBean(NoopTikeoWorkerClient.class);
                    Assertions.assertThat(scriptLanguages(noop)).doesNotContain("wasm");
                });
    }

    @Test
    void enablingScriptsDefaultsToWasmAndSrtShellWithoutContainerRuntime() throws Exception {
        installFakeWasmtime(stateDir);
        installFakeSrt(stateDir);
        installFakeRipgrep(stateDir);
        installFakePowerShell(stateDir);
        installFakeRhai(stateDir);
        contextRunner.withPropertyValues(
                "tikeo.worker.state-dir=" + stateDir,
                "tikeo.worker.scripts.enabled=true",
                "tikeo.worker.wasm.auto-install=false",
                "tikeo.worker.scripts.container-enabled=false",
                "tikeo.worker.scripts.auto-install-tools=false")
                .run(context -> {
                    NoopTikeoWorkerClient noop = context.getBean(NoopTikeoWorkerClient.class);
                    Assertions.assertThat(scriptLanguages(noop)).contains("wasm", "shell");
            Assertions.assertThat(scriptLanguages(noop)).doesNotContain("javascript", "typescript");
                });
    }

    @Test
    void enablingSandboxScriptsAdvertisesScriptCapabilitiesWhenRuntimeCheckIsDisabled() throws Exception {
        installFakeWasmtime(stateDir);
        installFakeSrt(stateDir);
        installFakeRipgrep(stateDir);
        installFakePowerShell(stateDir);
        installFakeRhai(stateDir);
        contextRunner.withPropertyValues(
                "tikeo.worker.state-dir=" + stateDir,
                "tikeo.worker.scripts.enabled=true",
                "tikeo.worker.scripts.container-enabled=true",
                "tikeo.worker.scripts.availability-check=false",
                "tikeo.worker.scripts.auto-install-tools=false",
                "tikeo.worker.scripts.runtime-command=docker",
                "tikeo.worker.scripts.images.shell=alpine:3.20",
                "tikeo.worker.scripts.images.python=python:3.13-alpine",
                "tikeo.worker.scripts.images.js=denoland/deno:alpine",
                "tikeo.worker.scripts.images.ts=denoland/deno:alpine",
                "tikeo.worker.scripts.images.powershell=mcr.microsoft.com/powershell:7.5-alpine-3.20",
                "tikeo.worker.scripts.images.php=php:8.4-cli-alpine",
                "tikeo.worker.scripts.images.groovy=groovy:4.0-jdk21",
                "tikeo.worker.scripts.images.rhai=rhaiscript/rhai:latest")
                .run(context -> {
                    NoopTikeoWorkerClient noop = context.getBean(NoopTikeoWorkerClient.class);
                    Assertions.assertThat(scriptLanguages(noop))
                            .contains("wasm", "shell");
                    Assertions.assertThat(scriptLanguages(noop)).doesNotContain("javascript", "typescript");
                    ScriptRunnerRegistry registry = context.getBean(ScriptRunnerRegistry.class);
                    Assertions.assertThat(registry.find(ScriptRunnerKind.SHELL))
                            .hasValueSatisfying(runner -> Assertions.assertThat(runner).isInstanceOf(SrtScriptRunner.class));
                });
    }

    @Test
    void sandboxScriptsStayWasmAndSrtShellWhenContainerRuntimeCommandIsMissing() throws Exception {
        installFakeWasmtime(stateDir);
        installFakeSrt(stateDir);
        installFakeRipgrep(stateDir);
        installFakePowerShell(stateDir);
        installFakeRhai(stateDir);
        contextRunner.withPropertyValues(
                "tikeo.worker.state-dir=" + stateDir,
                "tikeo.worker.scripts.enabled=true",
                "tikeo.worker.scripts.container-enabled=true",
                "tikeo.worker.scripts.availability-check=false",
                "tikeo.worker.scripts.auto-install-tools=false",
                "tikeo.worker.scripts.runtime-command=",
                "tikeo.worker.scripts.images.shell=alpine:3.20")
                .run(context -> {
                    NoopTikeoWorkerClient noop = context.getBean(NoopTikeoWorkerClient.class);
                    Assertions.assertThat(scriptLanguages(noop)).contains("wasm", "shell");
            Assertions.assertThat(scriptLanguages(noop)).doesNotContain("javascript", "typescript");
                });
    }

    @Test
    void unavailableContainerRuntimeKeepsDefaultWasmAndSrtShellOnly() throws Exception {
        installFakeWasmtime(stateDir);
        installFakeSrt(stateDir);
        installFakeRipgrep(stateDir);
        installFakePowerShell(stateDir);
        installFakeRhai(stateDir);
        contextRunner.withPropertyValues(
                "tikeo.worker.state-dir=" + stateDir,
                "tikeo.worker.scripts.enabled=true",
                "tikeo.worker.scripts.container-enabled=true",
                "tikeo.worker.scripts.availability-check=true",
                "tikeo.worker.scripts.auto-install-tools=false",
                "tikeo.worker.scripts.runtime-command=tikeo-missing-container-runtime")
                .run(context -> {
                    NoopTikeoWorkerClient noop = context.getBean(NoopTikeoWorkerClient.class);
                    Assertions.assertThat(scriptLanguages(noop)).contains("wasm", "shell");
            Assertions.assertThat(scriptLanguages(noop)).doesNotContain("javascript", "typescript");
                });
    }

    @Test
    void managementClientIsConditionalOnManagementFlag() {
        contextRunner.withPropertyValues(
                "tikeo.worker.state-dir=" + stateDir,
                "tikeo.management.enabled=true",
                "tikeo.management.endpoint=http://127.0.0.1:19999",
                "tikeo.management.api-key=tk-AbCdEfGhIjKlMnOpQrStUvWxYz0123456789AbCdEfGhIjKlMnOpQrStUv",
                "tikeo.management.namespace=demo-ns",
                "tikeo.management.app=demo-app").run(context -> {
            Assertions.assertThat(context).hasSingleBean(TikeoJobClient.class);
        });
    }

    @Test
    void managementClientIsDisabledByDefault() {
        contextRunner.withPropertyValues("tikeo.worker.state-dir=" + stateDir).run(context -> {
            Assertions.assertThat(context).doesNotHaveBean(TikeoJobClient.class);
        });
    }

    @Test
    void pluginProcessorCapabilityIsAdvertisedFromAnnotationMetadata() throws Exception {
        installFakeWasmtime(stateDir);
        log.info(() -> "[java-sdk-plugin-test] verifying worker registration advertises structured plugin processor metadata");

        contextRunner.withPropertyValues(
                "tikeo.worker.state-dir=" + stateDir,
                "tikeo.worker.capabilities[0]=java",
                "tikeo.worker.capabilities[1]=spring-boot",
                "tikeo.worker.labels.plugin.sql=enabled")
                .run(context -> {
                    NoopTikeoWorkerClient noop = context.getBean(NoopTikeoWorkerClient.class);
                    log.info(() -> "[java-sdk-plugin-test] registration capabilities="
                            + noop.registration().capabilities());
                    log.info(() -> "[java-sdk-plugin-test] structured capabilities="
                            + noop.registration().structuredCapabilities());
                    log.info(() -> "[java-sdk-plugin-test] registration labels="
                            + noop.registration().labels());
                    Assertions.assertThat(noop.registration().capabilities())
                            .containsExactly("java", "spring-boot");
                    Assertions.assertThat(noop.registration().structuredCapabilities().pluginProcessors())
                            .anySatisfy(plugin -> {
                                Assertions.assertThat(plugin.type()).isEqualTo("sql");
                                Assertions.assertThat(plugin.processorNames()).contains("billing.sql-sync");
                            });
                    Assertions.assertThat(noop.registration().labels()).containsEntry("plugin.sql", "enabled");
                });
    }

    @Test
    void autoStartupCanBeDisabledWhileKeepingClientBean() {
        contextRunner
                .withPropertyValues("tikeo.worker.state-dir=" + stateDir, "tikeo.worker.auto-startup=false")
                .run(context -> {
                    Assertions.assertThat(context).hasSingleBean(TikeoWorkerClient.class);
                    Assertions.assertThat(context).hasSingleBean(TikeoWorkerLifecycle.class);
                    NoopTikeoWorkerClient noop = context.getBean(NoopTikeoWorkerClient.class);
                    Assertions.assertThat(noop.running()).isFalse();
                });
    }

    @Test
    void disabledWorkerDoesNotCreateClientOrLifecycle() {
        contextRunner
                .withPropertyValues("tikeo.worker.enabled=false")
                .run(context -> {
                    Assertions.assertThat(context).doesNotHaveBean(TikeoWorkerClient.class);
                    Assertions.assertThat(context).doesNotHaveBean(TikeoWorkerLifecycle.class);
                    Assertions.assertThat(context).hasSingleBean(TikeoProcessorRegistry.class);
                });
    }

    private static void installFakeWasmtime(Path stateDir) throws Exception {
        Path binary = stateDir.resolve("sandbox-tools").resolve("wasmtime").resolve("bin").resolve("wasmtime");
        Files.createDirectories(binary.getParent());
        Files.writeString(binary, "#!/usr/bin/env sh\necho wasmtime 0.0.0-test\n");
        binary.toFile().setExecutable(true);
    }

    private static void installFakeSrt(Path stateDir) throws Exception {
        Path binary = stateDir.resolve("sandbox-tools").resolve("srt").resolve("bin").resolve("srt");
        Files.createDirectories(binary.getParent());
        Files.writeString(binary, "#!/usr/bin/env sh\necho srt 0.0.0-test\n");
        binary.toFile().setExecutable(true);
    }


    private static void installFakeRipgrep(Path stateDir) throws Exception {
        Path binary = stateDir.resolve("sandbox-tools").resolve("ripgrep").resolve("bin").resolve("rg");
        Files.createDirectories(binary.getParent());
        Files.writeString(binary, "#!/usr/bin/env sh\necho ripgrep 0.0.0-test\n");
        binary.toFile().setExecutable(true);
    }


    private static void installFakePowerShell(Path stateDir) throws Exception {
        Path binary = stateDir.resolve("sandbox-tools").resolve("pwsh").resolve("bin").resolve("pwsh");
        Files.createDirectories(binary.getParent());
        Files.writeString(binary, """
                #!/usr/bin/env sh
                echo PowerShell 7.5.4-test
                """);
        binary.toFile().setExecutable(true);
    }

    private static void installFakeRhai(Path stateDir) throws Exception {
        Path binary = stateDir.resolve("sandbox-tools").resolve("rhai").resolve("bin").resolve("rhai-run");
        Files.createDirectories(binary.getParent());
        Files.writeString(binary, """
                #!/usr/bin/env sh
                case "${1:-}" in
                  ""|"--version"|"-V")
                    echo rhai 1.0.0-test
                    exit 0
                    ;;
                esac
                test -f "${1:-}" && echo rhai 1.0.0-test
                """);
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
        @TikeoProcessor("demo.echo")
        public String echo(String payload) {
            return payload;
        }
    }

    static class DemoPluginProcessor {
        @TikeoProcessor(value = "billing.sql-sync", kind = TikeoProcessorKind.PLUGIN, pluginType = "sql")
        public String sync(String payload) {
            return payload;
        }
    }
}
