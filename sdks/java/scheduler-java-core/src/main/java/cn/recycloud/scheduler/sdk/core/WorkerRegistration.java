package cn.recycloud.scheduler.sdk.core;

import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * Worker metadata sent to scheduler during active outbound registration.
 */
public record WorkerRegistration(
        String workerId,
        String namespace,
        String app,
        String cluster,
        String region,
        List<String> capabilities,
        Map<String, String> labels) {

    public WorkerRegistration {
        Objects.requireNonNull(workerId, "workerId");
        Objects.requireNonNull(namespace, "namespace");
        Objects.requireNonNull(app, "app");
        Objects.requireNonNull(cluster, "cluster");
        Objects.requireNonNull(region, "region");
        capabilities = List.copyOf(capabilities == null ? List.of() : capabilities);
        labels = Map.copyOf(labels == null ? Map.of() : labels);
    }
}
