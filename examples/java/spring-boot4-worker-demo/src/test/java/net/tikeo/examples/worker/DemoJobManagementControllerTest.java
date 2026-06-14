package net.tikeo.examples.worker;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.List;
import net.tikeo.management.client.TikeoJobClient;
import net.tikeo.management.model.CreateJobRequest;
import net.tikeo.management.model.JobDefinition;
import net.tikeo.management.model.JobInstance;
import net.tikeo.management.model.JobRetryPolicy;
import net.tikeo.management.model.TriggerJobRequest;
import net.tikeo.management.model.UpdateJobRequest;
import org.assertj.core.api.Assertions;
import org.junit.jupiter.api.Test;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.boot.test.context.TestConfiguration;
import org.springframework.boot.test.web.server.LocalServerPort;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Primary;

@SpringBootTest(webEnvironment = SpringBootTest.WebEnvironment.RANDOM_PORT, properties = {
        "tikeo.worker.dry-run=true",
        "tikeo.worker.wasm.auto-install=false",
        "tikeo.worker.state-dir=${java.io.tmpdir}/spring-boot4-worker-demo-management-test",
        "tikeo.management.enabled=true"
})
class DemoJobManagementControllerTest {
    private static final Logger log = LoggerFactory.getLogger(DemoJobManagementControllerTest.class);
    @LocalServerPort
    private int port;

    @Test
    void exposesApiTypeTaskManagementExample() throws Exception {
        String list = httpGet("/demo/jobs");
        Assertions.assertThat(list).contains("demo.echo").contains("\"scheduleType\":\"api\"");

        String example = httpPost("/demo/jobs/echo");
        Assertions.assertThat(example)
                .contains("demo managed echo")
                .contains("\"scheduleType\":\"api\"")
                .contains("\"triggerType\":\"api\"")
                .contains("inst-demo");

        String scriptExample = httpPost("/demo/jobs/script/script-demo");
        Assertions.assertThat(scriptExample)
                .contains("demo managed script")
                .contains("\"scriptId\":\"script-demo\"")
                .contains("\"triggerType\":\"api\"");

        String pluginExample = httpPost("/demo/jobs/plugin/sql");
        log.info("[java-demo-plugin-test] plugin management response={}", pluginExample);
        Assertions.assertThat(pluginExample)
                .contains("demo managed sql plugin")
                .contains("\"processorType\":\"sql\"")
                .contains("\"processorName\":\"billing.sql-sync\"")
                .contains("\"triggerType\":\"api\"");
    }

    private String httpGet(String path) throws Exception {
        var request = HttpRequest.newBuilder(URI.create("http://localhost:" + port + path)).GET().build();
        var response = HttpClient.newHttpClient().send(request, HttpResponse.BodyHandlers.ofString());
        Assertions.assertThat(response.statusCode()).isEqualTo(200);
        return response.body();
    }

    private String httpPost(String path) throws Exception {
        var request = HttpRequest.newBuilder(URI.create("http://localhost:" + port + path)).POST(HttpRequest.BodyPublishers.noBody()).build();
        var response = HttpClient.newHttpClient().send(request, HttpResponse.BodyHandlers.ofString());
        Assertions.assertThat(response.statusCode()).isEqualTo(200);
        return response.body();
    }

    @TestConfiguration(proxyBeanMethods = false)
    static class FakeManagementClientConfig {
        @Bean
        @Primary
        TikeoJobClient fakeTikeoJobClient() {
            return new FakeTikeoJobClient();
        }
    }

    static final class FakeTikeoJobClient implements TikeoJobClient {
        @Override
        public List<JobDefinition> listJobs() {
            return List.of(job("job-demo", "demo echo", "demo.echo", true));
        }

        @Override
        public JobDefinition createJob(CreateJobRequest request) {
            Assertions.assertThat(request.scheduleType()).isEqualTo("api");
            if (request.processorType() != null) {
                log.info("[java-demo-plugin-test] fake create plugin job name={} processorType={} processorName={}",
                        request.name(), request.processorType(), request.processorName());
                Assertions.assertThat(request.processorType()).isEqualTo("sql");
                Assertions.assertThat(request.processorName()).isEqualTo("billing.sql-sync");
            } else if (request.scriptId() == null) {
                Assertions.assertThat(request.processorName()).isEqualTo("demo.echo");
            } else {
                Assertions.assertThat(request.processorName()).isNull();
                Assertions.assertThat(request.scriptId()).isEqualTo("script-demo");
            }
            return new JobDefinition("job-created", "default", "default", request.name(), "api", null,
                    request.processorType(), request.processorName(), request.scriptId(), true, JobRetryPolicy.defaults());
        }

        @Override
        public JobDefinition updateJob(String jobId, UpdateJobRequest request) {
            return job(jobId, "demo managed echo", "demo.echo", request.enabled() == null || request.enabled());
        }

        @Override
        public void deleteJob(String jobId) {}

        @Override
        public JobInstance triggerJob(String jobId, TriggerJobRequest request) {
            Assertions.assertThat(request.triggerType()).isEqualTo("api");
            return new JobInstance("inst-demo", jobId, "pending", "api", "single", "now", "now");
        }

        private static JobDefinition job(String id, String name, String processorName, boolean enabled) {
            return new JobDefinition(id, "default", "default", name, "api", null, null, processorName, null, enabled, JobRetryPolicy.defaults());
        }
    }
}
