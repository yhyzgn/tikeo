package com.yhyzgn.tikee.worker.identity;

import java.io.IOException;
import java.io.UncheckedIOException;
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
        if (hasText(explicitClientInstanceId)) {
            return explicitClientInstanceId.trim();
        }
        Objects.requireNonNull(stateRoot, "stateRoot");
        Path path = stateRoot.resolve(safeSegment(namespace))
                .resolve(safeSegment(app))
                .resolve(safeSegment(cluster))
                .resolve(safeSegment(region))
                .resolve(FILE_NAME);
        try {
            if (Files.exists(path)) {
                String existing = Files.readString(path, StandardCharsets.UTF_8).trim();
                if (hasText(existing)) {
                    return existing;
                }
            }
            Files.createDirectories(path.getParent());
            String generated = "java-" + digest(namespace, app, cluster, region, System.getProperty("user.name", "unknown"), path.toAbsolutePath().toString())
                    .substring(0, 24);
            Files.writeString(path, generated + System.lineSeparator(), StandardCharsets.UTF_8);
            return generated;
        } catch (IOException error) {
            throw new UncheckedIOException("Failed to resolve tikee client instance id at " + path, error);
        }
    }

    private static Path defaultStateRoot() {
        String configured = System.getProperty("tikee.worker.state-dir");
        if (hasText(configured)) {
            return Path.of(configured.trim());
        }
        String home = System.getProperty("user.home");
        if (hasText(home)) {
            return Path.of(home, ".tikee", "workers");
        }
        return Path.of(System.getProperty("java.io.tmpdir"), "tikee", "workers");
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
