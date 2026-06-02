package com.yhyzgn.tikee.examples.worker.processor;

import com.yhyzgn.tikee.processor.TaskOutcome;
import com.yhyzgn.tikee.processor.TikeeProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Intentional failure task example for integration verification. */
@Slf4j
@Component
public final class FailingTaskProcessor {
    @TikeeProcessor("demo.fail")
    public TaskOutcome fail(String payload) {
        log.info("[demo.fail] received payload='{}'", payload);
        TaskOutcome outcome = TaskOutcome.failed("demo failure:" + payload);
        log.warn("[demo.fail] returning expected failure message='{}'", outcome.message());
        return outcome;
    }
}
