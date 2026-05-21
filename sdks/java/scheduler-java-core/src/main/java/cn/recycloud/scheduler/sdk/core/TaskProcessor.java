package cn.recycloud.scheduler.sdk.core;

/**
 * Processes one task dispatched through the Worker Tunnel.
 */
@FunctionalInterface
public interface TaskProcessor {
    /**
     * Process one task.
     *
     * @param context task context
     * @return task outcome
     * @throws Exception when processing fails; the client reports the failure message to scheduler
     */
    TaskOutcome process(TaskContext context) throws Exception;
}
