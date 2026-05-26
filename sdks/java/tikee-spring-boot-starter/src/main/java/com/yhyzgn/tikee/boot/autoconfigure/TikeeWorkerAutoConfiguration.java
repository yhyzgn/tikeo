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
        registerContainerRunner(registry, ScriptRunnerKind.SHELL, scripts.getShell());
        registerContainerRunner(registry, ScriptRunnerKind.PYTHON, scripts.getPython());
        registerContainerRunner(registry, ScriptRunnerKind.NODE, scripts.getNode());
        registerContainerRunner(registry, ScriptRunnerKind.POWERSHELL, scripts.getPowershell());
        return registry;
    }

    private static void registerContainerRunner(
            ScriptRunnerRegistry registry,
            ScriptRunnerKind kind,
            TikeeWorkerProperties.ContainerScriptRunnerProperties properties) {
        if (!properties.isEnabled()) {
            return;
        }
        registry.register(new ContainerScriptRunner(
                kind,
                properties.getRuntimeCommand(),
                properties.getImage(),
                properties.getRuntimeArgs()));
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
