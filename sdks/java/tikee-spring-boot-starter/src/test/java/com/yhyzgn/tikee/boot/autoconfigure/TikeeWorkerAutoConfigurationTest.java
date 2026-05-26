package com.yhyzgn.tikee.boot.autoconfigure;

import static org.assertj.core.api.Assertions.assertThat;

import com.yhyzgn.tikee.boot.lifecycle.TikeeWorkerLifecycle;
import com.yhyzgn.tikee.processor.TikeeProcessor;
import com.yhyzgn.tikee.spring.processor.TikeeProcessorRegistry;
import com.yhyzgn.tikee.worker.client.NoopTikeeWorkerClient;
import com.yhyzgn.tikee.worker.client.TikeeWorkerClient;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.springframework.boot.test.context.runner.ApplicationContextRunner;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

class TikeeWorkerAutoConfigurationTest {
    @TempDir
    Path stateDir;

    private final ApplicationContextRunner contextRunner = new ApplicationContextRunner()
            .withUserConfiguration(TikeeWorkerAutoConfiguration.class, ProcessorConfig.class)
            .withPropertyValues(
                    "tikee.worker.dry-run=true",
                    "tikee.worker.app=billing");

    @Test
    void dryRunCreatesNoopClientWithGeneratedRegistrationHint() {
        contextRunner.withPropertyValues("tikee.worker.state-dir=" + stateDir).run(context -> {
            assertThat(context).hasSingleBean(TikeeWorkerClient.class);
            TikeeWorkerClient client = context.getBean(TikeeWorkerClient.class);
            assertThat(client).isInstanceOf(NoopTikeeWorkerClient.class);
            NoopTikeeWorkerClient noop = (NoopTikeeWorkerClient) client;
            assertThat(noop.registration().clientInstanceId()).startsWith("java-");
            assertThat(noop.registration().app()).isEqualTo("billing");
            assertThat(noop.running()).isTrue();
            assertThat(context.getBean(TikeeProcessorRegistry.class).handlers()).containsKey("demo.echo");
        });
    }

    @Test
    void explicitClientInstanceIdOverridesGeneratedValue() {
        contextRunner.withPropertyValues(
                "tikee.worker.state-dir=" + stateDir,
                "tikee.worker.client-instance-id=test-instance").run(context -> {
            NoopTikeeWorkerClient noop = context.getBean(NoopTikeeWorkerClient.class);
            assertThat(noop.registration().clientInstanceId()).isEqualTo("test-instance");
        });
    }

    @Test
    void autoStartupCanBeDisabledWhileKeepingClientBean() {
        contextRunner
                .withPropertyValues("tikee.worker.state-dir=" + stateDir, "tikee.worker.auto-startup=false")
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
