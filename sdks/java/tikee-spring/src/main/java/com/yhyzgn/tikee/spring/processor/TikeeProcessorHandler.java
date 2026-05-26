package com.yhyzgn.tikee.spring.processor;

import com.yhyzgn.tikee.processor.TaskContext;
import com.yhyzgn.tikee.processor.TaskOutcome;

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
