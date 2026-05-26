package com.yhyzgn.tikee.management.client;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import com.yhyzgn.tikee.management.model.CreateJobRequest;
import com.yhyzgn.tikee.management.model.JobScheduleType;
import com.yhyzgn.tikee.management.model.TriggerJobRequest;
import com.yhyzgn.tikee.management.model.UpdateJobRequest;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.http.HttpClient;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class HttpTikeeJobClientTest {
    private HttpServer server;
    private List<RecordedRequest> requests;
    private TikeeJobClient client;

    @BeforeEach
    void setUp() throws Exception {
        requests = new ArrayList<>();
        server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/api/v1/jobs", this::handleJobs);
        server.start();
        client = new HttpTikeeJobClient(
                HttpClient.newHttpClient(),
                new ObjectMapper(),
                "http://127.0.0.1:" + server.getAddress().getPort(),
                "token-1",
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
        assertEquals("demo.echo", jobs.getFirst().processorName());

        var created = client.createJob(CreateJobRequest.api("echo", "demo.echo"));
        assertEquals("api", created.scheduleType());

        RecordedRequest create = requests.stream()
                .filter(request -> request.method().equals("POST") && request.path().equals("/api/v1/jobs"))
                .findFirst()
                .orElseThrow();
        assertEquals("Bearer token-1", create.authorization());
        assertEquals(true, create.body().contains("\"namespace\":\"default\""));
        assertEquals(true, create.body().contains("\"app\":\"demo-app\""));
        assertEquals(true, create.body().contains("\"scheduleType\":\"api\""));
    }

    @Test
    void supportsUpdateDisableTriggerAndDelete() {
        var disabled = client.disableJob("job-1");
        assertFalse(disabled.enabled());

        var updated = client.updateJob("job-1", new UpdateJobRequest("report", JobScheduleType.API.value(), null, "demo.report", true));
        assertEquals("demo.report", updated.processorName());

        var instance = client.triggerJob("job-1", TriggerJobRequest.api());
        assertEquals("api", instance.triggerType());

        client.deleteJob("job-1");
        assertEquals(true, requests.stream().anyMatch(request -> request.method().equals("DELETE") && request.path().equals("/api/v1/jobs/job-1")));
    }

    @Test
    void nonSuccessStatusRaisesException() {
        assertThrows(TikeeManagementException.class, () -> client.deleteJob("missing"));
    }

    private void handleJobs(HttpExchange exchange) throws IOException {
        String path = exchange.getRequestURI().getPath();
        String method = exchange.getRequestMethod();
        String body = new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8);
        requests.add(new RecordedRequest(method, path, exchange.getRequestHeaders().getFirst("authorization"), body));

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
            respond(exchange, 200, "{\"code\":0,\"message\":\"success\",\"data\":{\"id\":\"job-1\",\"namespace\":\"default\",\"app\":\"demo-app\",\"name\":\"echo\",\"scheduleType\":\"api\",\"scheduleExpr\":null,\"processorName\":\"demo.echo\",\"enabled\":true}}");
            return;
        }
        if (method.equals("PATCH")) {
            boolean enabled = !body.contains("\"enabled\":false");
            String processor = body.contains("demo.report") ? "demo.report" : "demo.echo";
            respond(exchange, 200, "{\"code\":0,\"message\":\"success\",\"data\":{\"id\":\"job-1\",\"namespace\":\"default\",\"app\":\"demo-app\",\"name\":\"echo\",\"scheduleType\":\"api\",\"scheduleExpr\":null,\"processorName\":\"" + processor + "\",\"enabled\":" + enabled + "}}");
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

    private record RecordedRequest(String method, String path, String authorization, String body) {}
}
