package cn.recycloud.scheduler.sdk.spring;

import cn.recycloud.scheduler.sdk.core.GrpcSchedulerWorkerClient;
import cn.recycloud.scheduler.sdk.core.NoopSchedulerWorkerClient;
import cn.recycloud.scheduler.sdk.core.SchedulerWorkerClient;
import cn.recycloud.scheduler.sdk.core.TaskOutcome;
import cn.recycloud.scheduler.sdk.core.WorkerRegistration;
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
    SchedulerWorkerClient schedulerWorkerClient(SchedulerWorkerProperties properties) {
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
                context -> TaskOutcome.succeeded(),
                Duration.ofMillis(properties.getHeartbeatIntervalMillis()));
    }

    @Bean
    @ConditionalOnMissingBean
    static SchedulerProcessorRegistry schedulerProcessorRegistry() {
        return new SchedulerProcessorRegistry();
    }
}
