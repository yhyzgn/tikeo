package com.yhyzgn.tikee.boot.autoconfigure;

import com.yhyzgn.tikee.boot.lifecycle.TikeeWorkerLifecycle;
import com.yhyzgn.tikee.management.client.HttpTikeeJobClient;
import com.yhyzgn.tikee.management.client.TikeeJobClient;
import com.yhyzgn.tikee.script.ContainerScriptRunner;
import com.yhyzgn.tikee.script.ScriptRunnerKind;
import com.yhyzgn.tikee.script.ScriptRunnerRegistry;
import com.yhyzgn.tikee.worker.identity.ClientInstanceIds;
import com.yhyzgn.tikee.worker.client.GrpcTikeeWorkerClient;
import com.yhyzgn.tikee.worker.client.NoopTikeeWorkerClient;
import com.yhyzgn.tikee.worker.client.TikeeWorkerClient;
import com.yhyzgn.tikee.worker.WorkerRegistration;
import com.yhyzgn.tikee.spring.processor.TikeeProcessorRegistry;
import com.yhyzgn.tikee.spring.worker.SpringTikeeTaskProcessor;
import java.time.Duration;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.concurrent.TimeUnit;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.boot.autoconfigure.AutoConfiguration;
import org.springframework.boot.autoconfigure.condition.ConditionalOnMissingBean;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.boot.context.properties.EnableConfigurationProperties;
import org.springframework.context.annotation.Bean;

/**
 * Auto-configuration for the tikee Spring Boot Starter.
 */
@AutoConfiguration
@EnableConfigurationProperties({TikeeWorkerProperties.class, TikeeManagementProperties.class})
public class TikeeWorkerAutoConfiguration {
    private static final Logger log = LoggerFactory.getLogger(TikeeWorkerAutoConfiguration.class);
    @Bean
    @ConditionalOnMissingBean
    @ConditionalOnProperty(prefix = "tikee.worker", name = "enabled", havingValue = "true", matchIfMissing = true)
    TikeeWorkerClient tikeeWorkerClient(
            TikeeWorkerProperties properties,
            TikeeProcessorRegistry processorRegistry,
            ScriptRunnerRegistry scriptRunnerRegistry) {
        String clientInstanceId = properties.getStateDir() == null || properties.getStateDir().isBlank()
                ? ClientInstanceIds.resolve(
                        properties.getClientInstanceId(),
                        properties.getNamespace(),
                        properties.getApp(),
                        properties.getCluster(),
                        properties.getRegion())
                : ClientInstanceIds.resolve(
                        properties.getClientInstanceId(),
                        properties.getNamespace(),
                        properties.getApp(),
                        properties.getCluster(),
                        properties.getRegion(),
                        java.nio.file.Path.of(properties.getStateDir()));
        var registration = new WorkerRegistration(
                clientInstanceId,
                properties.getNamespace(),
                properties.getApp(),
                properties.getCluster(),
                properties.getRegion(),
                workerCapabilities(properties, processorRegistry, scriptRunnerRegistry),
                properties.getLabels());
        if (properties.isDryRun()) {
            return new NoopTikeeWorkerClient(registration);
        }
        return new GrpcTikeeWorkerClient(
                properties.getEndpoint(),
                registration,
                new SpringTikeeTaskProcessor(processorRegistry),
                scriptRunnerRegistry,
                Duration.ofMillis(properties.getHeartbeatIntervalMillis()));
    }

    @Bean
    @ConditionalOnMissingBean
    @ConditionalOnProperty(prefix = "tikee.worker", name = "enabled", havingValue = "true", matchIfMissing = true)
    TikeeWorkerLifecycle tikeeWorkerLifecycle(TikeeWorkerClient client, TikeeWorkerProperties properties) {
        return new TikeeWorkerLifecycle(client, properties);
    }

    private static List<String> workerCapabilities(
            TikeeWorkerProperties properties,
            TikeeProcessorRegistry processorRegistry,
            ScriptRunnerRegistry scriptRunnerRegistry) {
        var capabilities = new LinkedHashSet<String>();
        capabilities.addAll(properties.getCapabilities());
        capabilities.addAll(processorRegistry.processorCapabilities());
        capabilities.addAll(scriptRunnerRegistry.capabilities());
        return new ArrayList<>(capabilities);
    }

    @Bean
    @ConditionalOnMissingBean
    @ConditionalOnProperty(prefix = "tikee.worker", name = "enabled", havingValue = "true", matchIfMissing = true)
    ScriptRunnerRegistry tikeeScriptRunnerRegistry(TikeeWorkerProperties properties) {
        ScriptRunnerRegistry registry = new ScriptRunnerRegistry();
        TikeeWorkerProperties.ScriptRunnerProperties scripts = properties.getScripts();
        if (scripts.isEnabled()) {
            if (!scripts.isAvailabilityCheck() || containerRuntimeAvailable(scripts.getRuntimeCommand())) {
                registerContainerRunner(
                        registry,
                        ScriptRunnerKind.SHELL,
                        scripts.getRuntimeCommand(),
                        scripts.getImages().getShell(),
                        scripts.getRuntimeArgs());
                registerContainerRunner(
                        registry,
                        ScriptRunnerKind.PYTHON,
                        scripts.getRuntimeCommand(),
                        scripts.getImages().getPython(),
                        scripts.getRuntimeArgs());
                registerContainerRunner(
                        registry,
                        ScriptRunnerKind.NODE,
                        scripts.getRuntimeCommand(),
                        scripts.getImages().getNode(),
                        scripts.getRuntimeArgs());
                registerContainerRunner(
                        registry,
                        ScriptRunnerKind.POWERSHELL,
                        scripts.getRuntimeCommand(),
                        scripts.getImages().getPowershell(),
                        scripts.getRuntimeArgs());
            } else {
                log.warn(
                        "tikee script sandbox is enabled but container runtime '{}' is unavailable; "
                                + "script capabilities will not be advertised",
                        scripts.getRuntimeCommand());
            }
        }
        return registry;
    }

    private static void registerContainerRunner(
            ScriptRunnerRegistry registry,
            ScriptRunnerKind kind,
            String runtimeCommand,
            String image,
            List<String> runtimeArgs) {
        if (image == null || image.isBlank()) {
            return;
        }
        registry.register(new ContainerScriptRunner(kind, runtimeCommand, image, runtimeArgs));
    }

    static boolean containerRuntimeAvailable(String runtimeCommand) {
        try {
            Process process = new ProcessBuilder(runtimeCommand, "info", "--format", "{{.ServerVersion}}")
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

    @Bean
    @ConditionalOnMissingBean
    @ConditionalOnProperty(prefix = "tikee.management", name = "enabled", havingValue = "true")
    TikeeJobClient tikeeJobClient(TikeeManagementProperties properties) {
        return new HttpTikeeJobClient(
                properties.getEndpoint(),
                properties.getToken(),
                properties.getNamespace(),
                properties.getApp());
    }

    @Bean
    @ConditionalOnMissingBean
    static TikeeProcessorRegistry tikeeProcessorRegistry() {
        return new TikeeProcessorRegistry();
    }
}
