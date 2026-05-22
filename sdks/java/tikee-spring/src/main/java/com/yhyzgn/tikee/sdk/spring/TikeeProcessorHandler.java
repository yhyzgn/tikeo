package com.yhyzgn.tikee.sdk.spring;

import com.yhyzgn.tikee.sdk.core.TaskContext;
import com.yhyzgn.tikee.sdk.core.TaskOutcome;

/**
 * Invocable tikee processor discovered from Spring beans.
 */
@FunctionalInterface
public interface TikeeProcessorHandler {
    /**
     * Invoke the processor for one task.
     *
     * @param context task context
     * @return task outcome
     */
    TaskOutcome invoke(TaskContext context);
}
