package net.tikeo.examples.worker;

import static org.assertj.core.api.Assertions.assertThat;

import net.tikeo.worker.client.NoopTikeoWorkerClient;
import net.tikeo.worker.client.TikeoWorkerClient;
import net.tikeo.processor.TaskContext;
import net.tikeo.spring.processor.TikeoProcessorRegistry;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.boot.test.web.server.LocalServerPort;

@SpringBootTest(webEnvironment = SpringBootTest.WebEnvironment.RANDOM_PORT, properties = {
        "tikeo.worker.dry-run=true",
        "tikeo.worker.wasm.auto-install=false",
        "tikeo.worker.state-dir=${java.io.tmpdir}/spring-boot4-worker-demo-test",
        "tikeo.worker.client-instance-id=spring-boot4-worker-demo-test",
        "tikeo.worker.namespace=demo-ns",
        "tikeo.worker.app=demo-app",
        "tikeo.worker.cluster=demo-cluster",
        "tikeo.worker.region=demo-region",
        "tikeo.worker.capabilities[0]=java",
        "tikeo.worker.capabilities[1]=spring-boot",
        "tikeo.worker.labels.worker_pool=demo-pool",
        "tikeo.worker.labels.runtime=java",
        "tikeo.worker.labels.demo=spring-boot4-worker-demo"
})
class SpringWorkerDemoApplicationTest {
    private static final Logger log = LoggerFactory.getLogger(SpringWorkerDemoApplicationTest.class);
    @Autowired
    private TikeoWorkerClient client;

    @Autowired
    private TikeoProcessorRegistry registry;

    @LocalServerPort
    private int port;

    @Test
    void dryRunClientUsesGeneratedIdentityAndStartsWithLifecycle() {
        assertThat(client).isInstanceOf(NoopTikeoWorkerClient.class);
        NoopTikeoWorkerClient noop = (NoopTikeoWorkerClient) client;

        assertThat(noop.running()).isTrue();
        assertThat(noop.workerId()).isEqualTo("dry-run-spring-boot4-worker-demo-test");
        assertThat(noop.registration().clientInstanceId()).isEqualTo("spring-boot4-worker-demo-test");
        assertThat(noop.registration().namespace()).isEqualTo("demo-ns");
        assertThat(noop.registration().app()).isEqualTo("demo-app");
        assertThat(noop.registration().cluster()).isEqualTo("demo-cluster");
        assertThat(noop.registration().region()).isEqualTo("demo-region");
        assertThat(noop.registration().capabilities()).containsExactly("java", "spring-boot");
        assertThat(noop.registration().structuredCapabilities().pluginProcessors())
                .anySatisfy(plugin -> {
                    assertThat(plugin.type()).isEqualTo("sql");
                    assertThat(plugin.processorNames()).contains("billing.sql-sync");
                });
        assertThat(noop.registration().labels()).containsEntry("worker_pool", "demo-pool")
                .containsEntry("runtime", "java")
                .containsEntry("demo", "spring-boot4-worker-demo");
        log.info("[java-demo-plugin-test] dry-run registration capabilities={}", noop.registration().capabilities());
        log.info("[java-demo-plugin-test] dry-run structured capabilities={}", noop.registration().structuredCapabilities());
        log.info("[java-demo-plugin-test] dry-run registration labels={}", noop.registration().labels());
    }

    @Test
    void exposesStandardSpringBootWebDemoEndpoints() throws Exception {
        var health = httpGet("/demo/health");
        var processors = httpGet("/demo/processors");

        assertThat(health).contains("\"status\":\"ok\"");
        assertThat(health).contains("\"connected\":true");
        assertThat(health).contains("\"workerId\":\"dry-run-spring-boot4-worker-demo-test\"");
        assertThat(health).contains("demo.echo", "demo.fail", "demo.workflow.step");
        assertThat(processors).contains("demo.echo", "demo.context", "demo.bytes", "demo.heartbeat", "demo.report", "demo.workflow.step", "demo.fail", "billing.sql-sync");
        assertThat(processors).doesNotContain("shell.test");
        log.info("[java-demo-plugin-test] /demo/health response={}", health);
        log.info("[java-demo-plugin-test] /demo/processors response={}", processors);
    }

    @Test
    void springRegistersEchoProcessorAndInvokesItThroughRegistry() {
        assertThat(registry.handlers()).containsKeys("demo.echo", "demo.context", "demo.bytes", "demo.heartbeat", "demo.report", "demo.workflow.step", "demo.fail", "billing.sql-sync");
        assertThat(registry.handlers()).doesNotContainKey("shell.test");

        var outcome = registry.invoke("demo.echo", new TaskContext(
                "job-1",
                "demo.echo",
                "instance-1",
                "hello".getBytes(StandardCharsets.UTF_8)));

        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("echo:hello");
    }

    @Test
    void springRegistersPluginSqlProcessorAndInvokesItThroughRegistry() {
        var payload = "{\"source\":\"demo-test\",\"records\":3}";
        log.info("[java-demo-plugin-test] invoking registry processor=billing.sql-sync payload={}", payload);

        var outcome = registry.invoke("billing.sql-sync", new TaskContext(
                "job-sql-plugin",
                "billing.sql-sync",
                "instance-sql-plugin",
                payload.getBytes(StandardCharsets.UTF_8)));

        log.info("[java-demo-plugin-test] plugin processor outcome success={} message={}",
                outcome.success(), outcome.message());
        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("sql-plugin-ok:" + payload);
    }

    private String httpGet(String path) throws Exception {
        var request = HttpRequest.newBuilder(URI.create("http://localhost:" + port + path)).GET().build();
        var response = HttpClient.newHttpClient().send(request, HttpResponse.BodyHandlers.ofString());
        assertThat(response.statusCode()).isEqualTo(200);
        return response.body();
    }
}
