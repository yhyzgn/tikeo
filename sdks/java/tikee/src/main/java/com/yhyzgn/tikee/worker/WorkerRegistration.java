package com.yhyzgn.tikee.worker;

import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * Worker metadata sent to tikee during active outbound registration.
 *
 * <p>The tikee assigns the authoritative worker id after registration.
 * {@code clientInstanceId} is only a stable client-side hint for observability
 * and reconnect correlation.
 */
public record WorkerRegistration(
        String clientInstanceId,
        String namespace,
        String app,
        String cluster,
        String region,
        List<String> capabilities,
        WorkerCapabilitySet structuredCapabilities,
        WorkerClusterElection election,
        Map<String, String> labels) {

    public WorkerRegistration(
            String clientInstanceId,
            String namespace,
            String app,
            String cluster,
            String region,
            List<String> capabilities,
            Map<String, String> labels) {
        this(
                clientInstanceId,
                namespace,
                app,
                cluster,
                region,
                capabilities,
                WorkerCapabilitySet.tags(capabilities),
                WorkerClusterElection.enabledByDefault(),
                labels);
    }

    public WorkerRegistration {
        Objects.requireNonNull(clientInstanceId, "clientInstanceId");
        Objects.requireNonNull(namespace, "namespace");
        Objects.requireNonNull(app, "app");
        Objects.requireNonNull(cluster, "cluster");
        Objects.requireNonNull(region, "region");
        capabilities = List.copyOf(capabilities == null ? List.of() : capabilities);
        structuredCapabilities = structuredCapabilities == null ? WorkerCapabilitySet.tags(capabilities) : structuredCapabilities;
        election = election == null ? WorkerClusterElection.enabledByDefault() : election;
        labels = Map.copyOf(labels == null ? Map.of() : labels);
    }
}
