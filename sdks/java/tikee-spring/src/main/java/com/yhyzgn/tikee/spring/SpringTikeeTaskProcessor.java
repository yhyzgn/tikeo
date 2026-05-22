package com.yhyzgn.tikee.spring;

import com.yhyzgn.tikee.core.TaskContext;
import com.yhyzgn.tikee.core.TaskOutcome;
import com.yhyzgn.tikee.core.TaskProcessor;
import lombok.RequiredArgsConstructor;

/**
 * Routes dispatched tasks to Spring {@code @TikeeProcessor} handlers.
 *
 * <p>Routes by explicit {@link TaskContext#processorName()}, falling back to job id in older clients.
 */
@RequiredArgsConstructor
public final class SpringTikeeTaskProcessor implements TaskProcessor {
    private final TikeeProcessorRegistry registry;

    @Override
    public TaskOutcome process(TaskContext context) {
        return registry.invoke(context.processorName(), context);
    }
}
