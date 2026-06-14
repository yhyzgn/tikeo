package net.tikeo.examples.worker;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import net.tikeo.processor.TaskContext;
import net.tikeo.spring.processor.TikeoProcessorRegistry;
import net.tikeo.worker.client.NoopTikeoWorkerClient;
import net.tikeo.worker.client.TikeoWorkerClient;
import org.assertj.core.api.Assertions;
import org.junit.jupiter.api.Test;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.boot.test.web.server.LocalServerPort;

@SpringBootTest(webEnvironment = SpringBootTest.WebEnvironment.RANDOM_PORT, properties = {
        "tikeo.worker.dry-run=true",
        "tikeo.worker.wasm.auto-install=false",
        "tikeo.worker.state-dir=${java.io.tmpdir}/spring-boot3-worker-demo-test",
        "tikeo.worker.client-instance-id=spring-boot3-worker-demo-test",
        "tikeo.worker.namespace=demo-ns",
        "tikeo.worker.app=demo-app",
        "tikeo.worker.cluster=demo-cluster",
        "tikeo.worker.region=demo-region",
        "tikeo.worker.capabilities[0]=java",
        "tikeo.worker.capabilities[1]=spring-boot",
        "tikeo.worker.labels.worker_pool=demo-pool",
        "tikeo.worker.labels.runtime=java",
        "tikeo.worker.labels.demo=spring-boot3-worker-demo"
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
        Assertions.assertThat(client).isInstanceOf(NoopTikeoWorkerClient.class);
        NoopTikeoWorkerClient noop = (NoopTikeoWorkerClient) client;

        Assertions.assertThat(noop.running()).isTrue();
        Assertions.assertThat(noop.workerId()).isEqualTo("dry-run-spring-boot3-worker-demo-test");
        Assertions.assertThat(noop.registration().clientInstanceId()).isEqualTo("spring-boot3-worker-demo-test");
        Assertions.assertThat(noop.registration().namespace()).isEqualTo("demo-ns");
        Assertions.assertThat(noop.registration().app()).isEqualTo("demo-app");
        Assertions.assertThat(noop.registration().cluster()).isEqualTo("demo-cluster");
        Assertions.assertThat(noop.registration().region()).isEqualTo("demo-region");
        Assertions.assertThat(noop.registration().capabilities()).containsExactly("java", "spring-boot");
        Assertions.assertThat(noop.registration().structuredCapabilities().pluginProcessors())
                .anySatisfy(plugin -> {
                    Assertions.assertThat(plugin.type()).isEqualTo("sql");
                    Assertions.assertThat(plugin.processorNames()).contains("billing.sql-sync");
                });
        Assertions.assertThat(noop.registration().labels()).containsEntry("worker_pool", "demo-pool")
                .containsEntry("runtime", "java")
                .containsEntry("demo", "spring-boot3-worker-demo");
        log.info("[java-demo-plugin-test] dry-run registration capabilities={}", noop.registration().capabilities());
        log.info("[java-demo-plugin-test] dry-run structured capabilities={}", noop.registration().structuredCapabilities());
        log.info("[java-demo-plugin-test] dry-run registration labels={}", noop.registration().labels());
    }

    @Test
    void exposesStandardSpringBootWebDemoEndpoints() throws Exception {
        var health = httpGet("/demo/health");
        var processors = httpGet("/demo/processors");

        Assertions.assertThat(health).contains("\"status\":\"ok\"");
        Assertions.assertThat(health).contains("\"connected\":true");
        Assertions.assertThat(health).contains("\"workerId\":\"dry-run-spring-boot3-worker-demo-test\"");
        Assertions.assertThat(health).contains("demo.echo", "demo.fail", "demo.exception", "demo.workflow.step");
        Assertions.assertThat(processors).contains("demo.echo", "demo.context", "demo.bytes", "demo.heartbeat", "demo.report", "demo.workflow.step", "demo.fail", "demo.exception", "billing.sql-sync");
        Assertions.assertThat(processors).doesNotContain("shell.test");
        log.info("[java-demo-plugin-test] /demo/health response={}", health);
        log.info("[java-demo-plugin-test] /demo/processors response={}", processors);
    }

    @Test
    void springRegistersEchoProcessorAndInvokesItThroughRegistry() {
        Assertions.assertThat(registry.handlers()).containsKeys("demo.echo", "demo.context", "demo.bytes", "demo.heartbeat", "demo.report", "demo.workflow.step", "demo.fail", "demo.exception", "billing.sql-sync");
        Assertions.assertThat(registry.handlers()).doesNotContainKey("shell.test");

        var outcome = registry.invoke("demo.echo", new TaskContext(
                "job-1",
                "demo.echo",
                "instance-1",
                "hello".getBytes(StandardCharsets.UTF_8)));

        Assertions.assertThat(outcome.success()).isTrue();
        Assertions.assertThat(outcome.message()).isEqualTo("echo:hello");
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
        Assertions.assertThat(outcome.success()).isTrue();
        Assertions.assertThat(outcome.message()).isEqualTo("sql-plugin-ok:" + payload);
    }

    private String httpGet(String path) throws Exception {
        var request = HttpRequest.newBuilder(URI.create("http://localhost:" + port + path)).GET().build();
        var response = HttpClient.newHttpClient().send(request, HttpResponse.BodyHandlers.ofString());
        Assertions.assertThat(response.statusCode()).isEqualTo(200);
        return response.body();
    }
}
