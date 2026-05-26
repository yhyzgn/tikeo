package com.yhyzgn.tikee.management.model;

import com.fasterxml.jackson.annotation.JsonInclude;

/** Request to create a job in the current namespace/app scope. */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record CreateJobRequest(
        String name,
        String scheduleType,
        String scheduleExpr,
        String processorName,
        Boolean enabled) {
    public static CreateJobRequest api(String name, String processorName) {
        return new CreateJobRequest(name, JobScheduleType.API.value(), null, processorName, true);
    }
}
