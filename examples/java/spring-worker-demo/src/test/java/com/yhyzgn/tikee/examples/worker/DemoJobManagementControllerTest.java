package com.yhyzgn.tikee.examples.worker;

import static org.assertj.core.api.Assertions.assertThat;

import com.yhyzgn.tikee.management.client.TikeeJobClient;
import com.yhyzgn.tikee.management.model.CreateJobRequest;
import com.yhyzgn.tikee.management.model.JobDefinition;
import com.yhyzgn.tikee.management.model.JobInstance;
import com.yhyzgn.tikee.management.model.TriggerJobRequest;
import com.yhyzgn.tikee.management.model.UpdateJobRequest;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.boot.test.context.TestConfiguration;
import org.springframework.boot.test.web.server.LocalServerPort;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Primary;

@SpringBootTest(webEnvironment = SpringBootTest.WebEnvironment.RANDOM_PORT, properties = {
        "tikee.worker.dry-run=true",
        "tikee.worker.state-dir=${java.io.tmpdir}/tikee-spring-worker-demo-management-test",
        "tikee.management.enabled=true"
})
class DemoJobManagementControllerTest {
    @LocalServerPort
    private int port;

    @Test
    void exposesApiTypeTaskManagementExample() throws Exception {
        String list = httpGet("/demo/jobs");
        assertThat(list).contains("demo.echo").contains("\"scheduleType\":\"api\"");

        String example = httpPost("/demo/jobs/echo");
        assertThat(example)
                .contains("demo managed echo")
                .contains("\"scheduleType\":\"api\"")
                .contains("\"triggerType\":\"api\"")
                .contains("inst-demo");
    }

    private String httpGet(String path) throws Exception {
        var request = HttpRequest.newBuilder(URI.create("http://localhost:" + port + path)).GET().build();
        var response = HttpClient.newHttpClient().send(request, HttpResponse.BodyHandlers.ofString());
        assertThat(response.statusCode()).isEqualTo(200);
        return response.body();
    }

    private String httpPost(String path) throws Exception {
        var request = HttpRequest.newBuilder(URI.create("http://localhost:" + port + path)).POST(HttpRequest.BodyPublishers.noBody()).build();
        var response = HttpClient.newHttpClient().send(request, HttpResponse.BodyHandlers.ofString());
        assertThat(response.statusCode()).isEqualTo(200);
        return response.body();
    }

    @TestConfiguration(proxyBeanMethods = false)
    static class FakeManagementClientConfig {
        @Bean
        @Primary
        TikeeJobClient fakeTikeeJobClient() {
            return new FakeTikeeJobClient();
        }
    }

    static final class FakeTikeeJobClient implements TikeeJobClient {
        @Override
        public List<JobDefinition> listJobs() {
            return List.of(job("job-demo", "demo echo", "demo.echo", true));
        }

        @Override
        public JobDefinition createJob(CreateJobRequest request) {
            assertThat(request.scheduleType()).isEqualTo("api");
            assertThat(request.processorName()).isEqualTo("demo.echo");
            return job("job-created", request.name(), request.processorName(), true);
        }

        @Override
        public JobDefinition updateJob(String jobId, UpdateJobRequest request) {
            return job(jobId, "demo managed echo", "demo.echo", request.enabled() == null || request.enabled());
        }

        @Override
        public void deleteJob(String jobId) {}

        @Override
        public JobInstance triggerJob(String jobId, TriggerJobRequest request) {
            assertThat(request.triggerType()).isEqualTo("api");
            return new JobInstance("inst-demo", jobId, "pending", "api", "single", "now", "now");
        }

        private static JobDefinition job(String id, String name, String processorName, boolean enabled) {
            return new JobDefinition(id, "default", "default", name, "api", null, processorName, enabled);
        }
    }
}
