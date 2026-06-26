package net.tikeo.management.client;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.JavaType;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.net.URI;
import java.net.URLEncoder;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.List;
import java.util.Objects;
import net.tikeo.management.model.ApiEnvelope;
import net.tikeo.management.model.CreateJobRequest;
import net.tikeo.management.model.JobDefinition;
import net.tikeo.management.model.JobInstance;
import net.tikeo.management.model.JobRetryPolicy;
import net.tikeo.management.model.Page;
import net.tikeo.management.model.TriggerJobRequest;
import net.tikeo.management.model.UpdateJobRequest;

/**
 * HTTP implementation of {@link TikeoJobClient}.
 */
public final class HttpTikeoJobClient implements TikeoJobClient {

    private static final TypeReference<ApiEnvelope<JobDefinition>> JOB_ENVELOPE = new TypeReference<>() {};
    private static final TypeReference<ApiEnvelope<JobInstance>> INSTANCE_ENVELOPE = new TypeReference<>() {};

    private final HttpClient http;
    private final ObjectMapper mapper;
    private final URI endpoint;
    private final String apiKey;
    private final String namespace;
    private final String app;

    public HttpTikeoJobClient(String endpoint, String apiKey, String namespace, String app) {
        this(HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(10)).build(), new ObjectMapper(), endpoint, apiKey, namespace, app);
    }

    HttpTikeoJobClient(HttpClient http, ObjectMapper mapper, String endpoint, String apiKey, String namespace, String app) {
        this.http = Objects.requireNonNull(http, "http");
        this.mapper = Objects.requireNonNull(mapper, "mapper");
        this.endpoint = URI.create(trimTrailingSlash(Objects.requireNonNull(endpoint, "endpoint")) + "/");
        this.apiKey = Objects.requireNonNull(apiKey, "apiKey");
        this.namespace = namespace == null || namespace.isBlank() ? "default" : namespace;
        this.app = app == null || app.isBlank() ? "default" : app;
    }

    @Override
    public List<JobDefinition> listJobs() {
        ApiEnvelope<Page<JobDefinition>> envelope = send("GET", "/jobs", null, mapper.getTypeFactory().constructParametricType(ApiEnvelope.class, mapper.getTypeFactory().constructParametricType(Page.class, JobDefinition.class)));
        return envelope
            .data()
            .items()
            .stream()
            .filter(job -> namespace.equals(job.namespace()) && app.equals(job.app()))
            .toList();
    }

    @Override
    public JobDefinition createJob(CreateJobRequest request) {
        return send("POST", "/jobs", scopedCreate(request), JOB_ENVELOPE).data();
    }

    @Override
    public JobDefinition updateJob(String jobId, UpdateJobRequest request) {
        return send("PATCH", "/jobs/" + encode(jobId), request, JOB_ENVELOPE).data();
    }

    @Override
    public void deleteJob(String jobId) {
        send("DELETE", "/jobs/" + encode(jobId), null, new TypeReference<ApiEnvelope<Object>>() {});
    }

    @Override
    public JobInstance triggerJob(String jobId, TriggerJobRequest request) {
        return send("POST", "/jobs/" + encode(jobId) + ":trigger", request, INSTANCE_ENVELOPE).data();
    }

    private CreateJobPayload scopedCreate(CreateJobRequest request) {
        Objects.requireNonNull(request, "request");
        return new CreateJobPayload(namespace, app, request.name(), request.scheduleType(), request.scheduleExpr(), request.processorType(), request.processorName(), request.workerPool(), request.scriptId(), request.enabled(), request.retryPolicy());
    }

    private <T> T send(String method, String path, Object body, TypeReference<T> type) {
        JavaType javaType = mapper.getTypeFactory().constructType(type);
        return send(method, path, body, javaType);
    }

    private <T> T send(String method, String path, Object body, JavaType type) {
        try {
            HttpRequest.Builder builder = HttpRequest.newBuilder(endpoint.resolve("api/v1" + path))
                .timeout(Duration.ofSeconds(30))
                .header("x-tikeo-api-key", apiKey)
                .header("accept", "application/json");
            if (body == null) {
                builder.method(method, HttpRequest.BodyPublishers.noBody());
            } else {
                builder.header("content-type", "application/json").method(method, HttpRequest.BodyPublishers.ofString(mapper.writeValueAsString(body)));
            }
            HttpResponse<String> response = http.send(builder.build(), HttpResponse.BodyHandlers.ofString());
            if (response.statusCode() / 100 != 2) {
                throw new TikeoManagementException("tikeo management request failed: status=" + response.statusCode() + " body=" + response.body());
            }
            T envelope = mapper.readValue(response.body(), type);
            if (envelope instanceof ApiEnvelope<?> apiEnvelope && apiEnvelope.code() != 0) {
                throw new TikeoManagementException("tikeo management request failed: " + apiEnvelope.message());
            }
            return envelope;
        } catch (IOException error) {
            throw new TikeoManagementException("tikeo management request failed", error);
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            throw new TikeoManagementException("tikeo management request interrupted", error);
        }
    }

    private static String encode(String value) {
        return URLEncoder.encode(Objects.requireNonNull(value, "value"), StandardCharsets.UTF_8);
    }

    private static String trimTrailingSlash(String value) {
        String trimmed = value.trim();
        while (trimmed.endsWith("/")) {
            trimmed = trimmed.substring(0, trimmed.length() - 1);
        }
        return trimmed;
    }

    private record CreateJobPayload(String namespace, String app, String name, String scheduleType, String scheduleExpr, String processorType, String processorName, String workerPool, String scriptId, Boolean enabled, JobRetryPolicy retryPolicy) {}
}
