package cn.recycloud.scheduler.sdk.core;

import java.util.Arrays;
import java.util.Objects;

/**
 * Task dispatch context delivered by the scheduler Worker Tunnel.
 */
public record TaskContext(String jobId, String processorName, String instanceId, byte[] payload) {
    public TaskContext {
        Objects.requireNonNull(jobId, "jobId");
        processorName = (processorName == null || processorName.isBlank()) ? jobId : processorName;
        Objects.requireNonNull(instanceId, "instanceId");
        payload = payload == null ? new byte[0] : Arrays.copyOf(payload, payload.length);
    }

    /**
     * Backward-compatible constructor that uses {@code jobId} as processor name.
     *
     * @param jobId job id
     * @param instanceId instance id
     * @param payload raw payload
     */
    public TaskContext(String jobId, String instanceId, byte[] payload) {
        this(jobId, jobId, instanceId, payload);
    }

    @Override
    public byte[] payload() {
        return Arrays.copyOf(payload, payload.length);
    }
}
