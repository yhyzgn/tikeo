package net.tikeo.management.client;

import java.time.Instant;
import net.tikeo.management.model.CreateJobRequest;
import net.tikeo.management.model.TriggerJobRequest;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

class HttpTikeoJobClientLiveTest {

    @Test
    void usesLiveSdkApiKeyAndRejectsOutOfScopeApp() {
        String endpoint = env("TIKEO_LIVE_MANAGEMENT_ENDPOINT");
        String apiKey = env("TIKEO_LIVE_MANAGEMENT_API_KEY");
        Assumptions.assumeTrue(!endpoint.isBlank(), "TIKEO_LIVE_MANAGEMENT_ENDPOINT is required for live smoke");
        Assumptions.assumeTrue(!apiKey.isBlank(), "TIKEO_LIVE_MANAGEMENT_API_KEY is required for live smoke");

        String namespace = envOrDefault("TIKEO_LIVE_MANAGEMENT_NAMESPACE", "default");
        String app = envOrDefault("TIKEO_LIVE_MANAGEMENT_APP", "default");
        String otherApp = envOrDefault("TIKEO_LIVE_MANAGEMENT_OTHER_APP", "other");
        String jobName = "java-live-" + Instant.now().toEpochMilli();

        TikeoJobClient client = new HttpTikeoJobClient(endpoint, apiKey, namespace, app);
        var created = client.createJob(CreateJobRequest.api(jobName, "demo.echo"));
        try {
            Assertions.assertEquals(namespace, created.namespace());
            Assertions.assertEquals(app, created.app());
            Assertions.assertEquals(jobName, created.name());

            Assertions.assertTrue(client.listJobs().stream().anyMatch(job -> created.id().equals(job.id())));

            var triggered = client.triggerJob(created.id(), TriggerJobRequest.api());
            Assertions.assertEquals(created.id(), triggered.jobId());
            Assertions.assertEquals("api", triggered.triggerType());

            TikeoJobClient outOfScope = new HttpTikeoJobClient(endpoint, apiKey, namespace, otherApp);
            Assertions.assertThrows(
                    TikeoManagementException.class,
                    () -> outOfScope.createJob(CreateJobRequest.api(jobName + "-blocked", "demo.echo")));
        } finally {
            client.deleteJob(created.id());
        }
    }

    private static String env(String name) {
        return System.getenv(name) == null ? "" : System.getenv(name).trim();
    }

    private static String envOrDefault(String name, String fallback) {
        String value = env(name);
        return value.isBlank() ? fallback : value;
    }
}
