package net.tikeo.management.model;

import com.fasterxml.jackson.annotation.JsonInclude;

/**
 * Request to create a job in the current namespace/app scope.
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record CreateJobRequest(
        String name,
        String scheduleType,
        String scheduleExpr,
        String processorType,
        String processorName,
        String workerPool,
        String scriptId,
        Boolean enabled,
        JobRetryPolicy retryPolicy) {
    public CreateJobRequest(
            String name,
            String scheduleType,
            String scheduleExpr,
            String processorType,
            String processorName,
            String scriptId,
            Boolean enabled) {
        this(name, scheduleType, scheduleExpr, processorType, processorName, null, scriptId, enabled, null);
    }

    public static CreateJobRequest api(String name, String processorName) {
        return new CreateJobRequest(name, JobScheduleType.API.value(), null, null, processorName, null, null, true, JobRetryPolicy.defaults());
    }

    public static CreateJobRequest apiScript(String name, String scriptId) {
        return new CreateJobRequest(name, JobScheduleType.API.value(), null, null, null, null, scriptId, true, JobRetryPolicy.defaults());
    }

    public static CreateJobRequest apiPlugin(String name, String processorType, String processorName) {
        return new CreateJobRequest(name, JobScheduleType.API.value(), null, processorType, processorName, null, null, true, JobRetryPolicy.defaults());
    }

    public static CreateJobRequest cronPlugin(
            String name,
            String scheduleExpr,
            String processorType,
            String processorName) {
        return new CreateJobRequest(name, JobScheduleType.CRON.value(), scheduleExpr, processorType, processorName, null, null, true, JobRetryPolicy.defaults());
    }

    public CreateJobRequest withWorkerPool(String workerPool) {
        if (workerPool == null || workerPool.isBlank()) {
            return this;
        }
        return new CreateJobRequest(name, scheduleType, scheduleExpr, processorType, processorName, workerPool, scriptId, enabled, retryPolicy);
    }
}
