package com.yhyzgn.tikee.processor;

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
     * @throws Exception when processing fails; the client reports the failure message to tikee
     */
    TaskOutcome process(TaskContext context) throws Exception;
}
