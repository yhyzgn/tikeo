package com.yhyzgn.tikee.examples.worker.processor;

import com.yhyzgn.tikee.processor.TaskContext;
import com.yhyzgn.tikee.processor.TaskOutcome;
import com.yhyzgn.tikee.processor.TikeeProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Broadcast/context-aware task example. */
@Slf4j
@Component
public final class ContextTaskProcessor {
    @TikeeProcessor("demo.context")
    public TaskOutcome context(TaskContext context) {
        log.info("[demo.context] received jobId={} instanceId={} processor={}",
                context.jobId(), context.instanceId(), context.processorName());
        TaskOutcome outcome = new TaskOutcome(true, "context:" + context.processorName() + ":" + context.instanceId());
        log.info("[demo.context] completed instanceId={} message='{}'", context.instanceId(), outcome.message());
        return outcome;
    }
}
