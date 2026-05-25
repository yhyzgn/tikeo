package com.yhyzgn.tikee.examples.worker;

import static org.assertj.core.api.Assertions.assertThat;

import com.yhyzgn.tikee.core.NoopTikeeWorkerClient;
import com.yhyzgn.tikee.core.TikeeWorkerClient;
import com.yhyzgn.tikee.core.TaskContext;
import com.yhyzgn.tikee.spring.TikeeProcessorRegistry;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;

@SpringBootTest(properties = {
        "tikee.worker.demo.block-on-startup=false",
        "tikee.worker.dry-run=true",
        "tikee.worker.client-instance-id=test-spring-demo-worker",
        "tikee.worker.namespace=demo-ns",
        "tikee.worker.app=demo-app",
        "tikee.worker.cluster=demo-cluster",
        "tikee.worker.region=demo-region",
        "tikee.worker.capabilities[0]=java",
        "tikee.worker.capabilities[1]=spring-boot",
        "tikee.worker.labels.runtime=java",
        "tikee.worker.labels.demo=spring-worker"
})
class SpringWorkerDemoApplicationTest {
    @Autowired
    private TikeeWorkerClient client;

    @Autowired
    private TikeeProcessorRegistry registry;

    @Test
    void dryRunClientUsesConfiguredIdentityAndStartsWithLifecycle() {
        assertThat(client).isInstanceOf(NoopTikeeWorkerClient.class);
        NoopTikeeWorkerClient noop = (NoopTikeeWorkerClient) client;

        assertThat(noop.running()).isTrue();
        assertThat(noop.workerId()).isEqualTo("dry-run-test-spring-demo-worker");
        assertThat(noop.registration().clientInstanceId()).isEqualTo("test-spring-demo-worker");
        assertThat(noop.registration().namespace()).isEqualTo("demo-ns");
        assertThat(noop.registration().app()).isEqualTo("demo-app");
        assertThat(noop.registration().cluster()).isEqualTo("demo-cluster");
        assertThat(noop.registration().region()).isEqualTo("demo-region");
        assertThat(noop.registration().capabilities()).containsExactly("java", "spring-boot");
        assertThat(noop.registration().labels()).containsEntry("runtime", "java")
                .containsEntry("demo", "spring-worker");
    }

    @Test
    void springRegistersEchoProcessorAndInvokesItThroughRegistry() {
        assertThat(registry.handlers()).containsKey("demo.echo");

        var outcome = registry.invoke("demo.echo", new TaskContext(
                "job-1",
                "demo.echo",
                "instance-1",
                "hello".getBytes(StandardCharsets.UTF_8)));

        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("echo:hello");
    }
}
