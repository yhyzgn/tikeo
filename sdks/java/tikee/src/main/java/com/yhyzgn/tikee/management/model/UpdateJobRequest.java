package com.yhyzgn.tikee.management.model;

import com.fasterxml.jackson.annotation.JsonInclude;

/** Request to update a job. Omitted fields are unchanged. */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record UpdateJobRequest(
        String name,
        String scheduleType,
        String scheduleExpr,
        String processorType,
        String processorName,
        String scriptId,
        Boolean enabled,
        JobRetryPolicy retryPolicy) {
    public UpdateJobRequest(
            String name,
            String scheduleType,
            String scheduleExpr,
            String processorType,
            String processorName,
            String scriptId,
            Boolean enabled) {
        this(name, scheduleType, scheduleExpr, processorType, processorName, scriptId, enabled, null);
    }

    public static UpdateJobRequest disable() {
        return new UpdateJobRequest(null, null, null, null, null, null, false, null);
    }

    public static UpdateJobRequest enable() {
        return new UpdateJobRequest(null, null, null, null, null, null, true, null);
    }

    public static UpdateJobRequest apiPlugin(String name, String processorType, String processorName) {
        return new UpdateJobRequest(name, JobScheduleType.API.value(), null, processorType, processorName, null, true, null);
    }

    public static UpdateJobRequest cronPlugin(
            String name,
            String scheduleExpr,
            String processorType,
            String processorName) {
        return new UpdateJobRequest(name, JobScheduleType.CRON.value(), scheduleExpr, processorType, processorName, null, true, null);
    }
}
