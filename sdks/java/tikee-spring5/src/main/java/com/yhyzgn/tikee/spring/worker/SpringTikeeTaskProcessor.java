package com.yhyzgn.tikee.spring.worker;

import com.yhyzgn.tikee.processor.ProcessorCapabilityProvider;
import com.yhyzgn.tikee.processor.TaskContext;
import com.yhyzgn.tikee.processor.TaskOutcome;
import com.yhyzgn.tikee.processor.TaskProcessor;
import com.yhyzgn.tikee.spring.processor.TikeeProcessorRegistry;
import com.yhyzgn.tikee.worker.WorkerCapabilityProvider;
import com.yhyzgn.tikee.worker.WorkerCapabilitySet;
import java.util.List;
import lombok.RequiredArgsConstructor;

/**
 * Routes dispatched tasks to Spring {@code @TikeeProcessor} handlers.
 *
 * <p>Routes by explicit {@link TaskContext#processorName()}, falling back to job id in older clients.
 */
@RequiredArgsConstructor
public final class SpringTikeeTaskProcessor implements TaskProcessor, ProcessorCapabilityProvider, WorkerCapabilityProvider {
    private final TikeeProcessorRegistry registry;

    @Override
    public TaskOutcome process(TaskContext context) {
        return registry.invoke(context.processorName(), context);
    }

    @Override
    public List<String> capabilities() {
        return registry.processorCapabilities();
    }

    @Override
    public WorkerCapabilitySet workerCapabilities() {
        return registry.workerCapabilities();
    }
}
