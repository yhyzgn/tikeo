package cn.recycloud.scheduler.sdk.spring;

import cn.recycloud.scheduler.sdk.core.TaskContext;
import cn.recycloud.scheduler.sdk.core.TaskOutcome;

/**
 * Invocable scheduler processor discovered from Spring beans.
 */
@FunctionalInterface
public interface SchedulerProcessorHandler {
    /**
     * Invoke the processor for one task.
     *
     * @param context task context
     * @return task outcome
     */
    TaskOutcome invoke(TaskContext context);
}
