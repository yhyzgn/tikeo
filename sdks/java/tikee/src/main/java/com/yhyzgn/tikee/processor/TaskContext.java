package com.yhyzgn.tikee.processor;

import java.util.Arrays;
import java.util.Objects;

/**
 * Task dispatch context delivered by the tikee Worker Tunnel.
 */
public record TaskContext(String jobId, String processorName, String instanceId, byte[] payload, TaskLogger logger) {
    public TaskContext {
        Objects.requireNonNull(jobId, "jobId");
        processorName = (processorName == null || processorName.isBlank()) ? jobId : processorName;
        Objects.requireNonNull(instanceId, "instanceId");
        payload = payload == null ? new byte[0] : Arrays.copyOf(payload, payload.length);
        logger = logger == null ? TaskLogger.NOOP : logger;
    }

    public TaskContext(String jobId, String processorName, String instanceId, byte[] payload) {
        this(jobId, processorName, instanceId, payload, TaskLogger.NOOP);
    }

    /**
     * Backward-compatible constructor that uses {@code jobId} as processor name.
     *
     * @param jobId job id
     * @param instanceId instance id
     * @param payload raw payload
     */
    public TaskContext(String jobId, String instanceId, byte[] payload) {
        this(jobId, jobId, instanceId, payload, TaskLogger.NOOP);
    }

    public void logInfo(String message) {
        logger.info(message);
    }

    public void logError(String message) {
        logger.error(message);
    }

    @Override
    public byte[] payload() {
        return Arrays.copyOf(payload, payload.length);
    }
}
