package cn.recycloud.scheduler.sdk.spring;

import static org.assertj.core.api.Assertions.assertThat;

import cn.recycloud.scheduler.sdk.core.NoopSchedulerWorkerClient;
import cn.recycloud.scheduler.sdk.core.SchedulerWorkerClient;
import org.junit.jupiter.api.Test;
import org.springframework.boot.test.context.runner.ApplicationContextRunner;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import cn.recycloud.scheduler.sdk.core.SchedulerProcessor;

class SchedulerWorkerAutoConfigurationTest {
    private final ApplicationContextRunner contextRunner = new ApplicationContextRunner()
            .withUserConfiguration(SchedulerWorkerAutoConfiguration.class, ProcessorConfig.class)
            .withPropertyValues(
                    "scheduler.worker.dry-run=true",
                    "scheduler.worker.client-instance-id=test-instance",
                    "scheduler.worker.app=billing");

    @Test
    void dryRunCreatesNoopClientWithRegistrationHint() {
        contextRunner.run(context -> {
            assertThat(context).hasSingleBean(SchedulerWorkerClient.class);
            SchedulerWorkerClient client = context.getBean(SchedulerWorkerClient.class);
            assertThat(client).isInstanceOf(NoopSchedulerWorkerClient.class);
            NoopSchedulerWorkerClient noop = (NoopSchedulerWorkerClient) client;
            assertThat(noop.registration().clientInstanceId()).isEqualTo("test-instance");
            assertThat(noop.registration().app()).isEqualTo("billing");
            assertThat(context.getBean(SchedulerProcessorRegistry.class).handlers()).containsKey("demo.echo");
        });
    }

    @Configuration(proxyBeanMethods = false)
    static class ProcessorConfig {
        @Bean
        DemoProcessor demoProcessor() {
            return new DemoProcessor();
        }
    }

    static class DemoProcessor {
        @SchedulerProcessor("demo.echo")
        public String echo(String payload) {
            return payload;
        }
    }
}
