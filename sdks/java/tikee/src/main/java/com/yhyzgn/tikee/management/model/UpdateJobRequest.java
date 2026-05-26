package com.yhyzgn.tikee.management.model;

import com.fasterxml.jackson.annotation.JsonInclude;

/** Request to update a job. Omitted fields are unchanged. */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record UpdateJobRequest(
        String name,
        String scheduleType,
        String scheduleExpr,
        String processorName,
        Boolean enabled) {
    public static UpdateJobRequest disable() {
        return new UpdateJobRequest(null, null, null, null, false);
    }

    public static UpdateJobRequest enable() {
        return new UpdateJobRequest(null, null, null, null, true);
    }
}
