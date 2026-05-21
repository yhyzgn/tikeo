package cn.recycloud.scheduler.sdk.core;

import java.util.Arrays;
import java.util.Objects;

/**
 * Task dispatch context delivered by the scheduler Worker Tunnel.
 */
public record TaskContext(String jobId, String instanceId, byte[] payload) {
    public TaskContext {
        Objects.requireNonNull(jobId, "jobId");
        Objects.requireNonNull(instanceId, "instanceId");
        payload = payload == null ? new byte[0] : Arrays.copyOf(payload, payload.length);
    }

    @Override
    public byte[] payload() {
        return Arrays.copyOf(payload, payload.length);
    }
}
