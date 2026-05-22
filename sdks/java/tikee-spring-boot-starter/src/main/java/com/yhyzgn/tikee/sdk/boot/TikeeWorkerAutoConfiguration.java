package com.yhyzgn.tikee.sdk.boot;

import com.yhyzgn.tikee.sdk.core.GrpcTikeeWorkerClient;
import com.yhyzgn.tikee.sdk.core.NoopTikeeWorkerClient;
import com.yhyzgn.tikee.sdk.core.TikeeWorkerClient;
import com.yhyzgn.tikee.sdk.core.WorkerRegistration;
import com.yhyzgn.tikee.sdk.spring.TikeeProcessorRegistry;
import com.yhyzgn.tikee.sdk.spring.SpringTikeeTaskProcessor;
import java.time.Duration;
import org.springframework.boot.autoconfigure.AutoConfiguration;
import org.springframework.boot.autoconfigure.condition.ConditionalOnMissingBean;
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
    static TikeeProcessorRegistry tikeeProcessorRegistry() {
        return new TikeeProcessorRegistry();
    }
}
