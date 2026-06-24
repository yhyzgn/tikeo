package net.tikeo.examples.worker.processor;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TikeoProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Broadcast/context-aware task example. */
@Slf4j
@Component
public final class ContextTaskProcessor {
    @TikeoProcessor(value = "demo.context", description = "展示 TaskContext 元数据读取")
    public TaskOutcome context(TaskContext context) {
        log.info("[demo.context] received jobId={} instanceId={} processor={}",
                context.jobId(), context.instanceId(), context.processorName());
        TaskOutcome outcome = new TaskOutcome(true, "context:" + context.processorName() + ":" + context.instanceId());
        log.info("[demo.context] completed instanceId={} message='{}'", context.instanceId(), outcome.message());
        return outcome;
    }
}
