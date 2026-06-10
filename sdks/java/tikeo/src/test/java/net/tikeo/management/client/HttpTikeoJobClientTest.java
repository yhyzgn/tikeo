package net.tikeo.management.client;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import net.tikeo.management.model.BroadcastSelectorRequest;
import net.tikeo.management.model.CreateJobRequest;
import net.tikeo.management.model.JobScheduleType;
import net.tikeo.management.model.TriggerJobRequest;
import net.tikeo.management.model.UpdateJobRequest;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.http.HttpClient;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.logging.Logger;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class HttpTikeoJobClientTest {
    private static final Logger log = Logger.getLogger(HttpTikeoJobClientTest.class.getName());
    private HttpServer server;
    private List<RecordedRequest> requests;
    private TikeoJobClient client;

    @BeforeEach
    void setUp() throws Exception {
        requests = new ArrayList<>();
        server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/api/v1/jobs", this::handleJobs);
        server.start();
        client = new HttpTikeoJobClient(
                HttpClient.newHttpClient(),
                new ObjectMapper(),
                "http://127.0.0.1:" + server.getAddress().getPort(),
                "tk-AbCdEfGhIjKlMnOpQrStUvWxYz0123456789AbCdEfGhIjKlMnOpQrStUv",
                "default",
                "demo-app");
    }

    @AfterEach
    void tearDown() {
        server.stop(0);
    }

    @Test
    void scopesListAndCreateToConfiguredNamespaceAndApp() {
        var jobs = client.listJobs();
        assertEquals(1, jobs.size());
        assertEquals("demo.echo", jobs.get(0).processorName());

        var created = client.createJob(CreateJobRequest.api("echo", "demo.echo"));
        assertEquals("api", created.scheduleType());

        RecordedRequest create = requests.stream()
                .filter(request -> request.method().equals("POST") && request.path().equals("/api/v1/jobs"))
                .findFirst()
                .orElseThrow();
        assertEquals("tk-AbCdEfGhIjKlMnOpQrStUvWxYz0123456789AbCdEfGhIjKlMnOpQrStUv", create.apiKey());
        assertEquals(true, create.body().contains("\"namespace\":\"default\""));
        assertEquals(true, create.body().contains("\"app\":\"demo-app\""));
        assertEquals(true, create.body().contains("\"scheduleType\":\"api\""));
    }

    @Test
    void supportsUpdateDisableTriggerAndDelete() {
        var disabled = client.disableJob("job-1");
        assertFalse(disabled.enabled());

        var updated = client.updateJob("job-1", new UpdateJobRequest("report", JobScheduleType.API.value(), null, null, "demo.report", null, true));
        assertEquals("demo.report", updated.processorName());

        var instance = client.triggerJob("job-1", TriggerJobRequest.api());
        assertEquals("api", instance.triggerType());

        client.deleteJob("job-1");
        assertEquals(true, requests.stream().anyMatch(request -> request.method().equals("DELETE") && request.path().equals("/api/v1/jobs/job-1")));
    }

    @Test
    void supportsExplicitBroadcastApiTriggerSelector() {
        var instance = client.triggerJob(
                "job-1",
                TriggerJobRequest.broadcastApi(new BroadcastSelectorRequest(
                        List.of("manual-demo"),
                        "us-east-1",
                        "prod-a",
                        Map.of("worker_pool", "java-blue"))));

        assertEquals("api", instance.triggerType());

        RecordedRequest trigger = requests.stream()
                .filter(request -> request.method().equals("POST") && request.path().equals("/api/v1/jobs/job-1:trigger"))
                .reduce((first, second) -> second)
                .orElseThrow();
        assertTrue(trigger.body().contains("\"triggerType\":\"api\""));
        assertTrue(trigger.body().contains("\"executionMode\":\"broadcast\""));
        assertTrue(trigger.body().contains("\"broadcastSelector\""));
        assertTrue(trigger.body().contains("\"region\":\"us-east-1\""));
        assertTrue(trigger.body().contains("\"worker_pool\":\"java-blue\""));
    }

    @Test
    void createsAndUpdatesPluginProcessorJobsWithProcessorType() {
        log.info("[java-sdk-plugin-test] starting plugin processor job management test");

        var created = client.createJob(CreateJobRequest.apiPlugin(
                "sql sync",
                "sql",
                "billing.sql-sync"));
        log.info(() -> "[java-sdk-plugin-test] create response id=%s processorType=%s processorName=%s"
                .formatted(created.id(), created.processorType(), created.processorName()));
        assertEquals("sql", created.processorType());
        assertEquals("billing.sql-sync", created.processorName());

        var updated = client.updateJob("job-1", UpdateJobRequest.apiPlugin(
                "sql sync v2",
                "sql",
                "billing.sql-sync.v2"));
        log.info(() -> "[java-sdk-plugin-test] update response id=%s processorType=%s processorName=%s"
                .formatted(updated.id(), updated.processorType(), updated.processorName()));
        assertEquals("sql", updated.processorType());
        assertEquals("billing.sql-sync.v2", updated.processorName());

        RecordedRequest create = requests.stream()
                .filter(request -> request.method().equals("POST") && request.path().equals("/api/v1/jobs"))
                .reduce((first, second) -> second)
                .orElseThrow();
        log.info(() -> "[java-sdk-plugin-test] create request body=" + create.body());
        assertTrue(create.body().contains("\"processorType\":\"sql\""));
        assertTrue(create.body().contains("\"processorName\":\"billing.sql-sync\""));
        assertTrue(create.body().contains("\"namespace\":\"default\""));
        assertTrue(create.body().contains("\"app\":\"demo-app\""));

        RecordedRequest update = requests.stream()
                .filter(request -> request.method().equals("PATCH") && request.path().equals("/api/v1/jobs/job-1"))
                .reduce((first, second) -> second)
                .orElseThrow();
        log.info(() -> "[java-sdk-plugin-test] update request body=" + update.body());
        assertTrue(update.body().contains("\"processorType\":\"sql\""));
        assertTrue(update.body().contains("\"processorName\":\"billing.sql-sync.v2\""));
    }

    @Test
    void nonSuccessStatusRaisesException() {
        assertThrows(TikeoManagementException.class, () -> client.deleteJob("missing"));
    }

    private void handleJobs(HttpExchange exchange) throws IOException {
        String path = exchange.getRequestURI().getPath();
        String method = exchange.getRequestMethod();
        String body = new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8);
        requests.add(new RecordedRequest(method, path, exchange.getRequestHeaders().getFirst("x-tikeo-api-key"), body));

        if (path.endsWith("/missing")) {
            respond(exchange, 404, "{\"code\":404,\"message\":\"not found\",\"data\":null}");
            return;
        }
        if (method.equals("GET")) {
            respond(exchange, 200, "{\"code\":0,\"message\":\"success\",\"data\":{\"items\":[{\"id\":\"job-1\",\"namespace\":\"default\",\"app\":\"demo-app\",\"name\":\"echo\",\"scheduleType\":\"api\",\"scheduleExpr\":null,\"processorName\":\"demo.echo\",\"enabled\":true},{\"id\":\"job-2\",\"namespace\":\"default\",\"app\":\"other\",\"name\":\"other\",\"scheduleType\":\"api\",\"scheduleExpr\":null,\"processorName\":\"demo.echo\",\"enabled\":true}],\"nextPageToken\":null}}");
            return;
        }
        if (method.equals("POST") && path.endsWith(":trigger")) {
            respond(exchange, 200, "{\"code\":0,\"message\":\"success\",\"data\":{\"id\":\"inst-1\",\"jobId\":\"job-1\",\"status\":\"pending\",\"triggerType\":\"api\",\"executionMode\":\"single\",\"createdAt\":\"now\",\"updatedAt\":\"now\"}}");
            return;
        }
        if (method.equals("POST")) {
            String processorType = body.contains("\"processorType\":\"sql\"") ? "\"sql\"" : "null";
            String processor = body.contains("billing.sql-sync") ? "billing.sql-sync" : "demo.echo";
            respond(exchange, 200, "{\"code\":0,\"message\":\"success\",\"data\":{\"id\":\"job-1\",\"namespace\":\"default\",\"app\":\"demo-app\",\"name\":\"echo\",\"scheduleType\":\"api\",\"scheduleExpr\":null,\"processorType\":" + processorType + ",\"processorName\":\"" + processor + "\",\"enabled\":true}}");
            return;
        }
        if (method.equals("PATCH")) {
            boolean enabled = !body.contains("\"enabled\":false");
            String processor = body.contains("billing.sql-sync.v2")
                    ? "billing.sql-sync.v2"
                    : body.contains("demo.report") ? "demo.report" : "demo.echo";
            String processorType = body.contains("\"processorType\":\"sql\"") ? "\"sql\"" : "null";
            respond(exchange, 200, "{\"code\":0,\"message\":\"success\",\"data\":{\"id\":\"job-1\",\"namespace\":\"default\",\"app\":\"demo-app\",\"name\":\"echo\",\"scheduleType\":\"api\",\"scheduleExpr\":null,\"processorType\":" + processorType + ",\"processorName\":\"" + processor + "\",\"enabled\":" + enabled + "}}");
            return;
        }
        if (method.equals("DELETE")) {
            respond(exchange, 200, "{\"code\":0,\"message\":\"success\",\"data\":{}}");
            return;
        }
        respond(exchange, 405, "{}");
    }

    private static void respond(HttpExchange exchange, int status, String body) throws IOException {
        byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
        exchange.getResponseHeaders().set("content-type", "application/json");
        exchange.sendResponseHeaders(status, bytes.length);
        exchange.getResponseBody().write(bytes);
        exchange.close();
    }

    private record RecordedRequest(String method, String path, String apiKey, String body) {}
}
