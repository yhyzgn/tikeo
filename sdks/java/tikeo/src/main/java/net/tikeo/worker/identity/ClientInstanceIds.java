package net.tikeo.worker.identity;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.net.InetAddress;
import java.net.UnknownHostException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HexFormat;
import java.util.Locale;
import java.util.Objects;

/**
 * Utilities for deriving and persisting stable client-side worker instance ids.
 */
public final class ClientInstanceIds {
    private static final String FILE_NAME = "client-instance-id";

    private ClientInstanceIds() {}

    /**
     * Returns an explicit id when present, otherwise reads or creates a stable id under the default state directory.
     */
    public static String resolve(String explicitClientInstanceId, String namespace, String app, String cluster, String region) {
        return resolve(explicitClientInstanceId, namespace, app, cluster, region, defaultStateRoot());
    }

    /**
     * Returns an explicit id when present, otherwise reads or creates a stable id under {@code stateRoot}.
     */
    public static String resolve(
            String explicitClientInstanceId,
            String namespace,
            String app,
            String cluster,
            String region,
            Path stateRoot) {
        return resolve(explicitClientInstanceId, namespace, app, cluster, region, stateRoot, runtimeIdentity());
    }

    /**
     * Returns an explicit id when present, otherwise reads or creates a stable id scoped to a runtime identity.
     *
     * <p>The runtime identity is intentionally part of both the state path and generated id. In Kubernetes,
     * multiple Pods commonly share namespace/app/cluster/region but must register as separate worker
     * instances; using the Pod/host identity prevents those replicas from collapsing into one instance.
     */
    public static String resolve(
            String explicitClientInstanceId,
            String namespace,
            String app,
            String cluster,
            String region,
            Path stateRoot,
            String runtimeIdentity) {
        if (hasText(explicitClientInstanceId)) {
            return explicitClientInstanceId.trim();
        }
        Objects.requireNonNull(stateRoot, "stateRoot");
        String runtimeSegment = safeSegment(runtimeIdentity);
        Path path = stateRoot.resolve(safeSegment(namespace))
                .resolve(safeSegment(app))
                .resolve(safeSegment(cluster))
                .resolve(safeSegment(region))
                .resolve(runtimeSegment)
                .resolve(FILE_NAME);
        try {
            if (Files.exists(path)) {
                String existing = Files.readString(path, StandardCharsets.UTF_8).trim();
                if (hasText(existing)) {
                    return existing;
                }
            }
            Files.createDirectories(path.getParent());
            String generated = "java-" + digest(
                            namespace,
                            app,
                            cluster,
                            region,
                            runtimeSegment,
                            System.getProperty("user.name", "unknown"),
                            path.toAbsolutePath().toString())
                    .substring(0, 24);
            Files.writeString(path, generated + System.lineSeparator(), StandardCharsets.UTF_8);
            return generated;
        } catch (IOException error) {
            throw new UncheckedIOException("Failed to resolve tikeo client instance id at " + path, error);
        }
    }

    private static String runtimeIdentity() {
        String explicitRuntime = firstText(
                System.getenv("TIKEO_WORKER_RUNTIME_ID"),
                System.getenv("TIKEO_POD_NAME"),
                System.getenv("POD_NAME"),
                System.getenv("HOSTNAME"));
        if (hasText(explicitRuntime)) {
            return explicitRuntime.trim();
        }
        try {
            String hostName = InetAddress.getLocalHost().getHostName();
            if (hasText(hostName)) {
                return hostName.trim();
            }
        } catch (UnknownHostException ignored) {
            // Fall through to a process-local fallback.
        }
        return "pid-" + ProcessHandle.current().pid();
    }

    private static String firstText(String... values) {
        for (String value : values) {
            if (hasText(value)) {
                return value;
            }
        }
        return null;
    }

    private static Path defaultStateRoot() {
        String configured = System.getProperty("tikeo.worker.state-dir");
        if (hasText(configured)) {
            return Path.of(configured.trim());
        }
        String home = System.getProperty("user.home");
        if (hasText(home)) {
            return Path.of(home, ".tikeo", "workers");
        }
        return Path.of(System.getProperty("java.io.tmpdir"), "tikeo", "workers");
    }

    private static String safeSegment(String value) {
        String normalized = hasText(value) ? value.trim().toLowerCase(Locale.ROOT) : "default";
        return normalized.replaceAll("[^a-z0-9._-]", "_");
    }

    private static boolean hasText(String value) {
        return value != null && !value.isBlank();
    }

    private static String digest(String... values) {
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            for (String value : values) {
                digest.update((value == null ? "" : value).getBytes(StandardCharsets.UTF_8));
                digest.update((byte) 0);
            }
            return HexFormat.of().formatHex(digest.digest());
        } catch (NoSuchAlgorithmException error) {
            throw new IllegalStateException("SHA-256 is unavailable", error);
        }
    }
}
