package cn.recycloud.scheduler.sdk.spring;

import cn.recycloud.scheduler.sdk.core.TaskContext;
import cn.recycloud.scheduler.sdk.core.TaskOutcome;
import cn.recycloud.scheduler.sdk.core.TaskProcessor;
import lombok.RequiredArgsConstructor;

/**
 * Routes dispatched tasks to Spring {@code @SchedulerProcessor} handlers.
 *
 * <p>Current protocol convention: {@link TaskContext#jobId()} is treated as the processor name.
 */
@RequiredArgsConstructor
public final class SpringSchedulerTaskProcessor implements TaskProcessor {
    private final SchedulerProcessorRegistry registry;

    @Override
    public TaskOutcome process(TaskContext context) {
        return registry.invoke(context.jobId(), context);
    }
}
