package net.tikeo.management.model;

import com.fasterxml.jackson.annotation.JsonInclude;

/**
 * Request to trigger a job.
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record TriggerJobRequest(
        String triggerType,
        String executionMode,
        BroadcastSelectorRequest broadcastSelector) {
    public TriggerJobRequest(String triggerType, String executionMode) {
        this(triggerType, executionMode, null);
    }

    public static TriggerJobRequest api() {
        return new TriggerJobRequest(JobTriggerType.API.value(), ExecutionMode.SINGLE.value());
    }

    public static TriggerJobRequest broadcastApi(BroadcastSelectorRequest selector) {
        return new TriggerJobRequest(JobTriggerType.API.value(), ExecutionMode.BROADCAST.value(), selector);
    }
}
