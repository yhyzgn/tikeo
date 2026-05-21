package cn.recycloud.scheduler.sdk.spring;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import org.springframework.boot.context.properties.ConfigurationProperties;

/**
 * Spring Boot properties for scheduler workers.
 */
@ConfigurationProperties(prefix = "scheduler.worker")
public class SchedulerWorkerProperties {
    /** Scheduler Worker Tunnel endpoint. */
    private String endpoint = "http://0.0.0.0:9998";
    /** Dry-run mode avoids opening a live Worker Tunnel. */
    private boolean dryRun = false;
    /** Heartbeat interval in milliseconds. */
    private long heartbeatIntervalMillis = 10_000;
    /** Stable client-side instance hint; scheduler assigns the authoritative worker id. */
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

    public String getEndpoint() { return endpoint; }
    public void setEndpoint(String endpoint) { this.endpoint = endpoint; }
    public boolean isDryRun() { return dryRun; }
    public void setDryRun(boolean dryRun) { this.dryRun = dryRun; }
    public long getHeartbeatIntervalMillis() { return heartbeatIntervalMillis; }
    public void setHeartbeatIntervalMillis(long heartbeatIntervalMillis) { this.heartbeatIntervalMillis = heartbeatIntervalMillis; }
    public String getClientInstanceId() { return clientInstanceId; }
    public void setClientInstanceId(String clientInstanceId) { this.clientInstanceId = clientInstanceId; }
    public String getNamespace() { return namespace; }
    public void setNamespace(String namespace) { this.namespace = namespace; }
    public String getApp() { return app; }
    public void setApp(String app) { this.app = app; }
    public String getCluster() { return cluster; }
    public void setCluster(String cluster) { this.cluster = cluster; }
    public String getRegion() { return region; }
    public void setRegion(String region) { this.region = region; }
    public List<String> getCapabilities() { return capabilities; }
    public void setCapabilities(List<String> capabilities) { this.capabilities = capabilities; }
    public Map<String, String> getLabels() { return labels; }
    public void setLabels(Map<String, String> labels) { this.labels = labels; }
}
