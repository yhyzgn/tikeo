package com.yhyzgn.tikee.boot;

import static org.assertj.core.api.Assertions.assertThat;

import com.yhyzgn.tikee.core.NoopTikeeWorkerClient;
import com.yhyzgn.tikee.core.TikeeWorkerClient;
import com.yhyzgn.tikee.spring.TikeeProcessorRegistry;
import org.junit.jupiter.api.Test;
import org.springframework.boot.test.context.runner.ApplicationContextRunner;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import com.yhyzgn.tikee.core.TikeeProcessor;

class TikeeWorkerAutoConfigurationTest {
    private final ApplicationContextRunner contextRunner = new ApplicationContextRunner()
            .withUserConfiguration(TikeeWorkerAutoConfiguration.class, ProcessorConfig.class)
            .withPropertyValues(
                    "tikee.worker.dry-run=true",
                    "tikee.worker.client-instance-id=test-instance",
                    "tikee.worker.app=billing");

    @Test
    void dryRunCreatesNoopClientWithRegistrationHint() {
        contextRunner.run(context -> {
            assertThat(context).hasSingleBean(TikeeWorkerClient.class);
            TikeeWorkerClient client = context.getBean(TikeeWorkerClient.class);
            assertThat(client).isInstanceOf(NoopTikeeWorkerClient.class);
            NoopTikeeWorkerClient noop = (NoopTikeeWorkerClient) client;
            assertThat(noop.registration().clientInstanceId()).isEqualTo("test-instance");
            assertThat(noop.registration().app()).isEqualTo("billing");
            assertThat(noop.running()).isTrue();
            assertThat(context.getBean(TikeeProcessorRegistry.class).handlers()).containsKey("demo.echo");
        });
    }

    @Test
    void autoStartupCanBeDisabledWhileKeepingClientBean() {
        contextRunner
                .withPropertyValues("tikee.worker.auto-startup=false")
                .run(context -> {
                    assertThat(context).hasSingleBean(TikeeWorkerClient.class);
                    assertThat(context).hasSingleBean(TikeeWorkerLifecycle.class);
                    NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
                    assertThat(noop.running()).isFalse();
                });
    }

    @Test
    void disabledWorkerDoesNotCreateClientOrLifecycle() {
        contextRunner
                .withPropertyValues("tikee.worker.enabled=false")
                .run(context -> {
                    assertThat(context).doesNotHaveBean(TikeeWorkerClient.class);
                    assertThat(context).doesNotHaveBean(TikeeWorkerLifecycle.class);
                    assertThat(context).hasSingleBean(TikeeProcessorRegistry.class);
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
        @TikeeProcessor("demo.echo")
        public String echo(String payload) {
            return payload;
        }
    }
}
