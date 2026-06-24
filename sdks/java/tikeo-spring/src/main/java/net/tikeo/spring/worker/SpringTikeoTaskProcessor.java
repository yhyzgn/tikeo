package net.tikeo.spring.worker;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TaskProcessor;
import net.tikeo.spring.processor.TikeoProcessorRegistry;
import net.tikeo.worker.WorkerCapabilityProvider;
import net.tikeo.worker.WorkerCapabilitySet;
import lombok.RequiredArgsConstructor;

/**
 * Routes dispatched tasks to Spring {@code @TikeoProcessor} handlers.
 *
 * <p>Routes by explicit {@link TaskContext#processorName()}, falling back to job id in older clients.
 */
@RequiredArgsConstructor
public final class SpringTikeoTaskProcessor implements TaskProcessor, WorkerCapabilityProvider {
    private final TikeoProcessorRegistry registry;

    @Override
    public TaskOutcome process(TaskContext context) {
        return registry.invoke(context.processorName(), context);
    }

    @Override
    public WorkerCapabilitySet workerCapabilities() {
        return registry.workerCapabilities();
    }
}
