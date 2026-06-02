package com.yhyzgn.tikee.examples.worker;

import static org.assertj.core.api.Assertions.assertThat;

import com.yhyzgn.tikee.worker.client.TikeeWorkerClient;
import com.yhyzgn.tikee.spring.processor.TikeeProcessorRegistry;
import org.junit.jupiter.api.Test;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.context.ApplicationContext;

@SpringBootTest(properties = {"tikee.worker.enabled=false", "spring.main.web-application-type=none"})
class SpringWorkerDemoDisabledTest {
    @Autowired
    private ApplicationContext context;

    @Autowired
    private TikeeProcessorRegistry registry;

    @Test
    void disablingWorkerKeepsProcessorDiscoveryButDoesNotCreateClient() {
        assertThat(context.getBeansOfType(TikeeWorkerClient.class)).isEmpty();
        assertThat(registry.handlers()).containsKey("demo.echo");
    }
}
