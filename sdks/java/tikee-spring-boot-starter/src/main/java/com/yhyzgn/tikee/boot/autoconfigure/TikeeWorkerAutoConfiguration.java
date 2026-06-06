package com.yhyzgn.tikee.boot.autoconfigure;

import com.yhyzgn.tikee.boot.lifecycle.TikeeWorkerLifecycle;
import com.yhyzgn.tikee.management.client.HttpTikeeJobClient;
import com.yhyzgn.tikee.management.client.TikeeJobClient;
import com.yhyzgn.tikee.sandbox.SandboxToolResolver;
import com.yhyzgn.tikee.script.ContainerScriptRunner;
import com.yhyzgn.tikee.script.DenoScriptRunner;
import com.yhyzgn.tikee.script.ScriptRunnerKind;
import com.yhyzgn.tikee.script.SrtScriptRunner;
import com.yhyzgn.tikee.script.UnavailableScriptRunner;
import com.yhyzgn.tikee.script.ScriptRunnerRegistry;
import com.yhyzgn.tikee.spring.processor.TikeeProcessorRegistry;
import com.yhyzgn.tikee.spring.worker.SpringTikeeTaskProcessor;
import com.yhyzgn.tikee.wasm.CliWasmtimeRunner;
import com.yhyzgn.tikee.wasm.WasmRunnerRegistry;
import com.yhyzgn.tikee.worker.WorkerCapabilitySet;
import com.yhyzgn.tikee.worker.WorkerClusterElection;
import com.yhyzgn.tikee.worker.WorkerRegistration;
import com.yhyzgn.tikee.worker.client.GrpcTikeeWorkerClient;
import com.yhyzgn.tikee.worker.client.NoopTikeeWorkerClient;
import com.yhyzgn.tikee.worker.client.TikeeWorkerClient;
import com.yhyzgn.tikee.worker.identity.ClientInstanceIds;
import java.nio.file.Path;
import java.time.Duration;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.concurrent.TimeUnit;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.boot.autoconfigure.condition.ConditionalOnMissingBean;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.boot.context.properties.EnableConfigurationProperties;
import org.springframework.context.ApplicationContext;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

/**
 * Auto-configuration for the tikee Spring Boot Starter.
 */
@Configuration(proxyBeanMethods = false)
@EnableConfigurationProperties({
    TikeeWorkerProperties.class,
    TikeeManagementProperties.class,
})
public class TikeeWorkerAutoConfiguration {

    private static final Logger log = LoggerFactory.getLogger(
        TikeeWorkerAutoConfiguration.class
    );

    @Bean
    @ConditionalOnMissingBean
    @ConditionalOnProperty(
        prefix = "tikee.worker",
        name = "enabled",
        havingValue = "true",
        matchIfMissing = true
    )
    TikeeWorkerClient tikeeWorkerClient(
        TikeeWorkerProperties properties,
        TikeeProcessorRegistry processorRegistry,
        ScriptRunnerRegistry scriptRunnerRegistry,
        WasmRunnerRegistry wasmRunnerRegistry,
        ApplicationContext applicationContext
    ) {
        processorRegistry.scanExistingBeans(applicationContext);
        String clientInstanceId =
            properties.getStateDir() == null ||
            properties.getStateDir().isBlank()
                ? ClientInstanceIds.resolve(
                      properties.getClientInstanceId(),
                      properties.getNamespace(),
                      properties.getApp(),
                      properties.getCluster(),
                      properties.getRegion()
                  )
                : ClientInstanceIds.resolve(
                      properties.getClientInstanceId(),
                      properties.getNamespace(),
                      properties.getApp(),
                      properties.getCluster(),
                      properties.getRegion(),
                      java.nio.file.Path.of(properties.getStateDir())
                  );
        var registration = new WorkerRegistration(
            clientInstanceId,
            properties.getNamespace(),
            properties.getApp(),
            properties.getCluster(),
            properties.getRegion(),
            workerTags(properties),
            workerStructuredCapabilities(
                properties,
                processorRegistry,
                scriptRunnerRegistry,
                wasmRunnerRegistry
            ),
            new WorkerClusterElection(
                properties.getElection().isEnabled(),
                properties.getElection().getDomain(),
                properties.getElection().getPriority()
            ),
            properties.getLabels()
        );
        if (properties.isDryRun()) {
            return new NoopTikeeWorkerClient(registration);
        }
        return new GrpcTikeeWorkerClient(
            properties.getEndpoint(),
            registration,
            new SpringTikeeTaskProcessor(processorRegistry),
            scriptRunnerRegistry,
            wasmRunnerRegistry,
            Duration.ofMillis(properties.getHeartbeatIntervalMillis())
        );
    }

    @Bean
    @ConditionalOnMissingBean
    @ConditionalOnProperty(
        prefix = "tikee.worker",
        name = "enabled",
        havingValue = "true",
        matchIfMissing = true
    )
    TikeeWorkerLifecycle tikeeWorkerLifecycle(
        TikeeWorkerClient client,
        TikeeWorkerProperties properties
    ) {
        return new TikeeWorkerLifecycle(client, properties);
    }

    private static List<String> workerTags(TikeeWorkerProperties properties) {
        return new ArrayList<>(new LinkedHashSet<>(properties.getCapabilities()));
    }

    private static WorkerCapabilitySet workerStructuredCapabilities(
        TikeeWorkerProperties properties,
        TikeeProcessorRegistry processorRegistry,
        ScriptRunnerRegistry scriptRunnerRegistry,
        WasmRunnerRegistry wasmRunnerRegistry
    ) {
        return WorkerCapabilitySet.tags(properties.getCapabilities())
            .merge(processorRegistry.workerCapabilities())
            .merge(new WorkerCapabilitySet(
                List.of(),
                List.of(),
                scriptRunnerRegistry.structuredCapabilities(),
                List.of()
            ))
            .merge(new WorkerCapabilitySet(
                List.of(),
                List.of(),
                wasmRunnerRegistry.structuredCapabilities(),
                List.of()
            ));
    }

    @Bean
    @ConditionalOnMissingBean
    @ConditionalOnProperty(
        prefix = "tikee.worker",
        name = "enabled",
        havingValue = "true",
        matchIfMissing = true
    )
    WasmRunnerRegistry tikeeWasmRunnerRegistry(
        TikeeWorkerProperties properties
    ) {
        WasmRunnerRegistry registry = new WasmRunnerRegistry();
        if (!properties.getScripts().isEnabled()) {
            return registry;
        }
        sandboxToolResolver(properties)
            .resolveWasmtimeCommand()
            .ifPresentOrElse(
                runtimeCommand ->
                    registry.register(
                        new CliWasmtimeRunner(runtimeCommand, List.of())
                    ),
                () ->
                    log.warn(
                        "tikee default WASM sandbox runtime is unavailable; " +
                            "structured scriptRunners language=wasm will not be advertised"
                    )
            );
        return registry;
    }

    @Bean
    @ConditionalOnMissingBean
    @ConditionalOnProperty(
        prefix = "tikee.worker",
        name = "enabled",
        havingValue = "true",
        matchIfMissing = true
    )
    ScriptRunnerRegistry tikeeScriptRunnerRegistry(
        TikeeWorkerProperties properties
    ) {
        ScriptRunnerRegistry registry = new ScriptRunnerRegistry();
        TikeeWorkerProperties.ScriptRunnerProperties scripts =
            properties.getScripts();
        if (!scripts.isEnabled()) {
            return registry;
        }
        registerDefaultSrtScriptRunners(registry, properties);
        if (scripts.isContainerEnabled()) {
            if (
                scripts.getRuntimeCommand() == null ||
                scripts.getRuntimeCommand().isBlank()
            ) {
                log.warn(
                    "tikee non-WASM script runners are enabled but no container runtime command is configured; " +
                        "structured scriptRunners language=shell/python/javascript/typescript/powershell/php/groovy/rhai will not be advertised"
                );
            } else if (
                !scripts.isAvailabilityCheck() ||
                runtimeAvailable(
                    scripts.getRuntimeCommand(),
                    "info",
                    "--format",
                    "{{.ServerVersion}}"
                )
            ) {
                registerContainerRunner(
                    registry,
                    ScriptRunnerKind.SHELL,
                    scripts.getRuntimeCommand(),
                    scripts.getImages().getShell(),
                    scripts.getRuntimeArgs()
                );
                registerContainerRunner(
                    registry,
                    ScriptRunnerKind.PYTHON,
                    scripts.getRuntimeCommand(),
                    scripts.getImages().getPython(),
                    scripts.getRuntimeArgs()
                );
                registerContainerRunner(
                    registry,
                    ScriptRunnerKind.JS,
                    scripts.getRuntimeCommand(),
                    scripts.getImages().getJs(),
                    scripts.getRuntimeArgs()
                );
                registerContainerRunner(
                    registry,
                    ScriptRunnerKind.TS,
                    scripts.getRuntimeCommand(),
                    scripts.getImages().getTs(),
                    scripts.getRuntimeArgs()
                );
                registerContainerRunner(
                    registry,
                    ScriptRunnerKind.POWERSHELL,
                    scripts.getRuntimeCommand(),
                    scripts.getImages().getPowershell(),
                    scripts.getRuntimeArgs()
                );
                registerContainerRunner(
                    registry,
                    ScriptRunnerKind.PHP,
                    scripts.getRuntimeCommand(),
                    scripts.getImages().getPhp(),
                    scripts.getRuntimeArgs()
                );
                registerContainerRunner(
                    registry,
                    ScriptRunnerKind.GROOVY,
                    scripts.getRuntimeCommand(),
                    scripts.getImages().getGroovy(),
                    scripts.getRuntimeArgs()
                );
                registerContainerRunner(
                    registry,
                    ScriptRunnerKind.RHAI,
                    scripts.getRuntimeCommand(),
                    scripts.getImages().getRhai(),
                    scripts.getRuntimeArgs()
                );
            } else {
                log.warn(
                    "tikee script sandbox is enabled but container runtime '{}' is unavailable; " +
                        "script capabilities will not be advertised",
                    scripts.getRuntimeCommand()
                );
            }
        }
        return registry;
    }

    private static void registerDefaultSrtScriptRunners(
        ScriptRunnerRegistry registry,
        TikeeWorkerProperties properties
    ) {
        SandboxToolResolver resolver = sandboxToolResolver(properties);
        var srtCommand = resolver.resolveSrtCommand();
        var ripgrepCommand = resolver.resolveRipgrepCommand();
        if (srtCommand.isPresent() && ripgrepCommand.isPresent()) {
            registerSrtNativeRunners(registry, resolver, srtCommand.get(), ripgrepCommand.get());
        } else {
            registerUnavailableNativeRunners(
                registry,
                srtCommand.isEmpty()
                    ? "SRT sandbox runtime is unavailable"
                    : "SRT sandbox dependency ripgrep (rg) is unavailable"
            );
        }
        resolver
            .resolveDenoCommand()
            .ifPresentOrElse(
                runtimeCommand -> {
                    registry.register(new DenoScriptRunner(ScriptRunnerKind.JS, runtimeCommand));
                    registry.register(new DenoScriptRunner(ScriptRunnerKind.TS, runtimeCommand));
                },
                () -> {
                    registry.register(
                        new UnavailableScriptRunner(
                            ScriptRunnerKind.JS,
                            "Deno sandbox runtime is unavailable"
                        )
                    );
                    registry.register(
                        new UnavailableScriptRunner(
                            ScriptRunnerKind.TS,
                            "Deno sandbox runtime is unavailable"
                        )
                    );
                }
            );
    }

    private static void registerSrtNativeRunners(
        ScriptRunnerRegistry registry,
        SandboxToolResolver resolver,
        String runtimeCommand,
        String ripgrepCommand
    ) {
        for (ScriptRunnerKind kind : List.of(
            ScriptRunnerKind.SHELL,
            ScriptRunnerKind.PYTHON,
            ScriptRunnerKind.POWERSHELL,
            ScriptRunnerKind.PHP,
            ScriptRunnerKind.GROOVY,
            ScriptRunnerKind.RHAI
        )) {
            var interpreter = resolveSrtInterpreter(kind, resolver);
            if (interpreter.isEmpty()) {
                registry.register(new UnavailableScriptRunner(
                    kind,
                    kind.value() + " SRT interpreter is unavailable"
                ));
                continue;
            }
            registry.register(new SrtScriptRunner(
                kind,
                runtimeCommand,
                interpreter.get(),
                srtPathEntries(resolver, runtimeCommand, ripgrepCommand, interpreter.get())
            ));
        }
    }

    private static java.util.Optional<String> resolveSrtInterpreter(
        ScriptRunnerKind kind,
        SandboxToolResolver resolver
    ) {
        return switch (kind) {
            case SHELL -> resolver.resolveInterpreterCommand("sh");
            case PYTHON -> resolver.resolveInterpreterCommand("python3");
            case POWERSHELL -> resolver.resolvePowerShellCommand();
            case PHP -> resolver.resolveInterpreterCommand("php");
            case GROOVY -> resolver.resolveInterpreterCommand("groovy");
            case RHAI -> resolver.resolveRhaiCommand();
            case JS, TS -> resolver.resolveDenoCommand();
        };
    }

    private static List<String> srtPathEntries(
        SandboxToolResolver resolver,
        String runtimeCommand,
        String ripgrepCommand,
        String interpreterCommand
    ) {
        java.util.LinkedHashSet<String> entries = new java.util.LinkedHashSet<>();
        for (String command : List.of(runtimeCommand, ripgrepCommand, interpreterCommand)) {
            toolPathEntry(command).ifPresent(entries::add);
        }
        resolver.resolveNodeCommand().flatMap(TikeeWorkerAutoConfiguration::toolPathEntry).ifPresent(entries::add);
        resolver.resolveNpmCommand().flatMap(TikeeWorkerAutoConfiguration::toolPathEntry).ifPresent(entries::add);
        return List.copyOf(entries);
    }

    private static java.util.Optional<String> toolPathEntry(String command) {
        if (command == null || command.isBlank()) {
            return java.util.Optional.empty();
        }
        try {
            java.nio.file.Path parent = java.nio.file.Path.of(command).getParent();
            return parent == null ? java.util.Optional.empty() : java.util.Optional.of(parent.toString());
        } catch (Exception ignored) {
            return java.util.Optional.empty();
        }
    }

    private static void registerUnavailableNativeRunners(
        ScriptRunnerRegistry registry,
        String reason
    ) {
        for (ScriptRunnerKind kind : List.of(
            ScriptRunnerKind.SHELL,
            ScriptRunnerKind.PYTHON,
            ScriptRunnerKind.POWERSHELL,
            ScriptRunnerKind.PHP,
            ScriptRunnerKind.GROOVY,
            ScriptRunnerKind.RHAI
        )) {
            registry.register(new UnavailableScriptRunner(kind, reason));
        }
    }

    private static void registerContainerRunner(
        ScriptRunnerRegistry registry,
        ScriptRunnerKind kind,
        String runtimeCommand,
        String image,
        List<String> runtimeArgs
    ) {
        if (image == null || image.isBlank() || registry.find(kind).isPresent()) {
            return;
        }
        registry.register(
            new ContainerScriptRunner(kind, runtimeCommand, image, runtimeArgs)
        );
    }

    static boolean runtimeAvailable(String runtimeCommand, String... args) {
        try {
            List<String> command = new ArrayList<>();
            command.add(runtimeCommand);
            command.addAll(List.of(args));
            Process process = new ProcessBuilder(command)
                .redirectErrorStream(true)
                .start();
            if (!process.waitFor(2, TimeUnit.SECONDS)) {
                process.destroyForcibly();
                return false;
            }
            return process.exitValue() == 0;
        } catch (Exception error) {
            return false;
        }
    }

    private static SandboxToolResolver sandboxToolResolver(
        TikeeWorkerProperties properties
    ) {
        TikeeWorkerProperties.ScriptRunnerProperties scripts =
            properties.getScripts();
        TikeeWorkerProperties.WasmProperties wasm = properties.getWasm();
        return new SandboxToolResolver(
            new SandboxToolResolver.Options(
                properties.getStateDir(),
                wasm.isAutoInstall(),
                wasm.getInstallVersion(),
                wasm.getInstallDir(),
                wasm.getInstallerUrl(),
                scripts.isWasmedgeAutoInstall(),
                scripts.getWasmedgeInstallVersion(),
                scripts.getWasmedgeInstallDir(),
                scripts.getWasmedgeInstallerUrl(),
                scripts.isAutoInstallTools(),
                scripts.getSrtInstallVersion(),
                scripts.getSrtInstallDir(),
                scripts.getRipgrepInstallVersion(),
                scripts.getRipgrepInstallDir(),
                scripts.getDenoInstallVersion(),
                scripts.getDenoInstallDir(),
                scripts.getDenoInstallerUrl(),
                scripts.getV8InstallVersion(),
                scripts.getV8InstallDir(),
                scripts.getRhaiInstallVersion(),
                scripts.getRhaiInstallDir(),
                scripts.getPowerShellInstallVersion(),
                scripts.getPowerShellInstallDir(),
                Math.max(
                    wasm.getInstallTimeoutMillis(),
                    scripts.getToolInstallTimeoutMillis()
                )
            )
        );
    }

    @Bean
    @ConditionalOnMissingBean
    @ConditionalOnProperty(
        prefix = "tikee.management",
        name = "enabled",
        havingValue = "true"
    )
    TikeeJobClient tikeeJobClient(TikeeManagementProperties properties) {
        return new HttpTikeeJobClient(
            properties.getEndpoint(),
            properties.getApiKey(),
            properties.getNamespace(),
            properties.getApp()
        );
    }


    @Bean
    @ConditionalOnMissingBean
    static TikeeProcessorRegistry tikeeProcessorRegistry() {
        return new TikeeProcessorRegistry();
    }
}
