package cn.recycloud.scheduler.sdk.spring;

import cn.recycloud.scheduler.sdk.core.NoopSchedulerWorkerClient;
import cn.recycloud.scheduler.sdk.core.SchedulerWorkerClient;
import cn.recycloud.scheduler.sdk.core.WorkerRegistration;
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
    SchedulerWorkerClient schedulerWorkerClient(SchedulerWorkerProperties properties) {
        var registration = new WorkerRegistration(
                properties.getWorkerId(),
                properties.getNamespace(),
                properties.getApp(),
                properties.getCluster(),
                properties.getRegion(),
                properties.getCapabilities(),
                properties.getLabels());
        return new NoopSchedulerWorkerClient(registration);
    }

    @Bean
    @ConditionalOnMissingBean
    SchedulerProcessorRegistry schedulerProcessorRegistry() {
        return new SchedulerProcessorRegistry();
    }
}
