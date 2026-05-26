package com.yhyzgn.tikee.examples.worker;

import static org.assertj.core.api.Assertions.assertThat;

import com.yhyzgn.tikee.worker.client.NoopTikeeWorkerClient;
import com.yhyzgn.tikee.worker.client.TikeeWorkerClient;
import com.yhyzgn.tikee.processor.TaskContext;
import com.yhyzgn.tikee.spring.processor.TikeeProcessorRegistry;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.boot.test.web.server.LocalServerPort;

@SpringBootTest(webEnvironment = SpringBootTest.WebEnvironment.RANDOM_PORT, properties = {
        "tikee.worker.dry-run=true",
        "tikee.worker.wasm.auto-install=false",
        "tikee.worker.state-dir=${java.io.tmpdir}/tikee-spring-worker-demo-test",
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

    @LocalServerPort
    private int port;

    @Test
    void dryRunClientUsesGeneratedIdentityAndStartsWithLifecycle() {
        assertThat(client).isInstanceOf(NoopTikeeWorkerClient.class);
        NoopTikeeWorkerClient noop = (NoopTikeeWorkerClient) client;

        assertThat(noop.running()).isTrue();
        assertThat(noop.workerId()).startsWith("dry-run-java-");
        assertThat(noop.registration().clientInstanceId()).startsWith("java-");
        assertThat(noop.registration().namespace()).isEqualTo("demo-ns");
        assertThat(noop.registration().app()).isEqualTo("demo-app");
        assertThat(noop.registration().cluster()).isEqualTo("demo-cluster");
        assertThat(noop.registration().region()).isEqualTo("demo-region");
        assertThat(noop.registration().capabilities()).contains("java", "spring-boot");
        assertThat(noop.registration().labels()).containsEntry("runtime", "java")
                .containsEntry("demo", "spring-worker");
    }

    @Test
    void exposesStandardSpringBootWebDemoEndpoints() throws Exception {
        var health = httpGet("/demo/health");
        var processors = httpGet("/demo/processors");

        assertThat(health).contains("\"status\":\"ok\"");
        assertThat(health).contains("\"connected\":true");
        assertThat(health).contains("\"workerId\":\"dry-run-java-");
        assertThat(health).contains("demo.echo", "demo.fail", "demo.workflow.step");
        assertThat(processors).contains("demo.echo", "demo.context", "demo.bytes", "demo.heartbeat", "demo.report", "demo.workflow.step", "demo.fail");
        assertThat(processors).doesNotContain("shell.test");
    }

    @Test
    void springRegistersEchoProcessorAndInvokesItThroughRegistry() {
        assertThat(registry.handlers()).containsKeys("demo.echo", "demo.context", "demo.bytes", "demo.heartbeat", "demo.report", "demo.workflow.step", "demo.fail");
        assertThat(registry.handlers()).doesNotContainKey("shell.test");

        var outcome = registry.invoke("demo.echo", new TaskContext(
                "job-1",
                "demo.echo",
                "instance-1",
                "hello".getBytes(StandardCharsets.UTF_8)));

        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("echo:hello");
    }

    private String httpGet(String path) throws Exception {
        var request = HttpRequest.newBuilder(URI.create("http://localhost:" + port + path)).GET().build();
        var response = HttpClient.newHttpClient().send(request, HttpResponse.BodyHandlers.ofString());
        assertThat(response.statusCode()).isEqualTo(200);
        return response.body();
    }
}
