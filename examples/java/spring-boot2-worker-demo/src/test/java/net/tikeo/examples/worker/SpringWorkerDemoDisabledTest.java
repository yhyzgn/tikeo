package net.tikeo.examples.worker;

import static org.assertj.core.api.Assertions.assertThat;

import net.tikeo.worker.client.TikeoWorkerClient;
import net.tikeo.spring.processor.TikeoProcessorRegistry;
import org.junit.jupiter.api.Test;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.context.ApplicationContext;

@SpringBootTest(properties = {"tikeo.worker.enabled=false", "spring.main.web-application-type=none"})
class SpringWorkerDemoDisabledTest {
    @Autowired
    private ApplicationContext context;

    @Autowired
    private TikeoProcessorRegistry registry;

    @Test
    void disablingWorkerKeepsProcessorDiscoveryButDoesNotCreateClient() {
        assertThat(context.getBeansOfType(TikeoWorkerClient.class)).isEmpty();
        assertThat(registry.handlers()).containsKey("demo.echo");
    }
}
