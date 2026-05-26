package com.yhyzgn.tikee.boot.autoconfigure;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import lombok.Getter;
import lombok.Setter;
import org.springframework.boot.context.properties.ConfigurationProperties;

/**
 * Spring Boot properties for tikee workers.
 */
@Getter
@Setter
@ConfigurationProperties(prefix = "tikee.worker")
public class TikeeWorkerProperties {
    /** Enable tikee worker auto-configuration. */
    private boolean enabled = true;
    /** Auto-start the worker client with the Spring application lifecycle. */
    private boolean autoStartup = true;
    /** Tikee Worker Tunnel endpoint. */
    private String endpoint = "http://0.0.0.0:9998";
    /** Dry-run mode avoids opening a live Worker Tunnel. */
    private boolean dryRun = false;
    /** Heartbeat interval in milliseconds. */
    private long heartbeatIntervalMillis = 10_000;
    /** Stable client-side instance hint; tikee assigns the authoritative worker id. */
    private String clientInstanceId = "spring-worker";
    /** Namespace reported during registration. */
    private String namespace = "default";
    /** App reported during registration. */
    private String app = "default";
    /** Cluster reported during registration. */
    private String cluster = "default";
    /** Region reported during registration. */
    private String region = "default";
    /** Capabilities reported during registration. */
    private List<String> capabilities = new ArrayList<>();
    /** Labels reported during registration. */
    private Map<String, String> labels = new LinkedHashMap<>();
}
