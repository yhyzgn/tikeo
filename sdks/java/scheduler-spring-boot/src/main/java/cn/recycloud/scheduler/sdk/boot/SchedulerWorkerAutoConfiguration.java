package cn.recycloud.scheduler.sdk.boot;

import cn.recycloud.scheduler.sdk.core.GrpcSchedulerWorkerClient;
import cn.recycloud.scheduler.sdk.core.NoopSchedulerWorkerClient;
import cn.recycloud.scheduler.sdk.core.SchedulerWorkerClient;
import cn.recycloud.scheduler.sdk.core.WorkerRegistration;
import cn.recycloud.scheduler.sdk.spring.SchedulerProcessorRegistry;
import cn.recycloud.scheduler.sdk.spring.SpringSchedulerTaskProcessor;
import java.time.Duration;
import org.springframework.boot.autoconfigure.AutoConfiguration;
import org.springframework.boot.autoconfigure.condition.ConditionalOnMissingBean;
import org.springframework.boot.context.properties.EnableConfigurationProperties;
import org.springframework.context.annotation.Bean;

/**
 * Auto-configuration for the scheduler Spring Boot Starter.
 */
@AutoConfiguration
@EnableConfigurationProperties(SchedulerWorkerProperties.class)
public class SchedulerWorkerAutoConfiguration {
    @Bean
    @ConditionalOnMissingBean
    SchedulerWorkerClient schedulerWorkerClient(
            SchedulerWorkerProperties properties, SchedulerProcessorRegistry processorRegistry) {
        var registration = new WorkerRegistration(
                properties.getClientInstanceId(),
                properties.getNamespace(),
                properties.getApp(),
                properties.getCluster(),
                properties.getRegion(),
                properties.getCapabilities(),
                properties.getLabels());
        if (properties.isDryRun()) {
            return new NoopSchedulerWorkerClient(registration);
        }
        return new GrpcSchedulerWorkerClient(
                properties.getEndpoint(),
                registration,
                new SpringSchedulerTaskProcessor(processorRegistry),
                Duration.ofMillis(properties.getHeartbeatIntervalMillis()));
    }

    @Bean
    @ConditionalOnMissingBean
    static SchedulerProcessorRegistry schedulerProcessorRegistry() {
        return new SchedulerProcessorRegistry();
    }
}
