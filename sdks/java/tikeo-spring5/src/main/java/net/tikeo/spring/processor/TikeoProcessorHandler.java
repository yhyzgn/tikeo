package net.tikeo.spring.processor;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;

/**
 * Invocable tikeo processor discovered from Spring beans.
 */
@FunctionalInterface
public interface TikeoProcessorHandler {
    /**
     * Invoke the processor for one task.
     *
     * @param context task context
     * @return task outcome
     */
    TaskOutcome invoke(TaskContext context);
}
