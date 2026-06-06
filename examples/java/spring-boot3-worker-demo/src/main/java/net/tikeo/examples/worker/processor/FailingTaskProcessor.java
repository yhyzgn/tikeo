package net.tikeo.examples.worker.processor;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TikeoProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Intentional failure task example for integration verification. */
@Slf4j
@Component
public final class FailingTaskProcessor {
    @TikeoProcessor("demo.fail")
    public TaskOutcome fail(TaskContext context, String payload) {
        log.info("[demo.fail] received payload='{}'", payload);
        context.logError("[demo.fail] received payload='" + payload + "'");
        TaskOutcome outcome = TaskOutcome.failed("demo failure:" + payload);
        log.warn("[demo.fail] returning expected failure message='{}'", outcome.message());
        return outcome;
    }
}
