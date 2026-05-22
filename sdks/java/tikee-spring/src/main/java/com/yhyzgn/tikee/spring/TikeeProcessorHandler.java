package com.yhyzgn.tikee.spring;

import com.yhyzgn.tikee.core.TaskContext;
import com.yhyzgn.tikee.core.TaskOutcome;

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
