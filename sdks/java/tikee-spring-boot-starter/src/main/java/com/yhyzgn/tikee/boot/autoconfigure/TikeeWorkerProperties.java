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
    /** Optional stable client-side instance hint; when blank, the SDK generates and persists one per scope. */
    private String clientInstanceId;
    /** Directory used to persist generated client instance ids. Blank uses ~/.tikee/workers. */
    private String stateDir;
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
    /** Sandboxed script runner configuration. */
    private ScriptRunnerProperties scripts = new ScriptRunnerProperties();

    /** Container-backed sandbox script runners. */
    @Getter
    @Setter
    public static class ScriptRunnerProperties {
        /** POSIX shell sandbox runner. */
        private ContainerScriptRunnerProperties shell = new ContainerScriptRunnerProperties("alpine:3.20");
        /** Python sandbox runner. */
        private ContainerScriptRunnerProperties python = new ContainerScriptRunnerProperties("python:3.13-alpine");
        /** Node.js sandbox runner. */
        private ContainerScriptRunnerProperties node = new ContainerScriptRunnerProperties("node:24-alpine");
        /** PowerShell sandbox runner. */
        private ContainerScriptRunnerProperties powershell = new ContainerScriptRunnerProperties("mcr.microsoft.com/powershell:7.5-alpine-3.20");
    }

    /** Per-language container sandbox settings. */
    @Getter
    @Setter
    public static class ContainerScriptRunnerProperties {
        /** Enable this sandbox runner and advertise the matching script:<language> capability. */
        private boolean enabled = false;
        /** Docker-compatible container runtime command. */
        private String runtimeCommand = "docker";
        /** Container image containing the language runtime. */
        private String image;
        /** Extra runtime arguments appended before image. */
        private List<String> runtimeArgs = new ArrayList<>();

        public ContainerScriptRunnerProperties() {}

        public ContainerScriptRunnerProperties(String image) {
            this.image = image;
        }
    }
}
