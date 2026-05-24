package com.yhyzgn.tikee.boot;

import com.yhyzgn.tikee.core.GrpcTikeeWorkerClient;
import com.yhyzgn.tikee.core.NoopTikeeWorkerClient;
import com.yhyzgn.tikee.core.TikeeWorkerClient;
import com.yhyzgn.tikee.core.WorkerRegistration;
import com.yhyzgn.tikee.spring.TikeeProcessorRegistry;
import com.yhyzgn.tikee.spring.SpringTikeeTaskProcessor;
import java.time.Duration;
import org.springframework.boot.autoconfigure.AutoConfiguration;
import org.springframework.boot.autoconfigure.condition.ConditionalOnMissingBean;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.boot.context.properties.EnableConfigurationProperties;
import org.springframework.context.annotation.Bean;

/**
 * Auto-configuration for the tikee Spring Boot Starter.
 */
@AutoConfiguration
@EnableConfigurationProperties(TikeeWorkerProperties.class)
public class TikeeWorkerAutoConfiguration {
    @Bean
    @ConditionalOnMissingBean
    @ConditionalOnProperty(prefix = "tikee.worker", name = "enabled", havingValue = "true", matchIfMissing = true)
    TikeeWorkerClient tikeeWorkerClient(
            TikeeWorkerProperties properties, TikeeProcessorRegistry processorRegistry) {
        var registration = new WorkerRegistration(
                properties.getClientInstanceId(),
                properties.getNamespace(),
                properties.getApp(),
                properties.getCluster(),
                properties.getRegion(),
                properties.getCapabilities(),
                properties.getLabels());
        if (properties.isDryRun()) {
            return new NoopTikeeWorkerClient(registration);
        }
        return new GrpcTikeeWorkerClient(
                properties.getEndpoint(),
                registration,
                new SpringTikeeTaskProcessor(processorRegistry),
                Duration.ofMillis(properties.getHeartbeatIntervalMillis()));
    }

    @Bean
    @ConditionalOnMissingBean
    @ConditionalOnProperty(prefix = "tikee.worker", name = "enabled", havingValue = "true", matchIfMissing = true)
    TikeeWorkerLifecycle tikeeWorkerLifecycle(TikeeWorkerClient client, TikeeWorkerProperties properties) {
        return new TikeeWorkerLifecycle(client, properties);
    }

    @Bean
    @ConditionalOnMissingBean
    static TikeeProcessorRegistry tikeeProcessorRegistry() {
        return new TikeeProcessorRegistry();
    }
}
