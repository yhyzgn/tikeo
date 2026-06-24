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
    @TikeoProcessor(value = "demo.fail", description = "返回失败结果的演示处理器")
    public TaskOutcome fail(TaskContext context, String payload) {
        log.error("[demo.fail] received payload='{}'", payload);
        TaskOutcome outcome = TaskOutcome.failed("demo failure:" + payload);
        log.warn("[demo.fail] returning expected failure message='{}'", outcome.message());
        return outcome;
    }

    @TikeoProcessor(value = "demo.exception", description = "抛出异常用于失败链路演示")
    public TaskOutcome exception(TaskContext context, String payload) {
        log.error("[demo.exception] throwing runtime exception payload='{}'", payload);
        throw new IllegalStateException("java demo runtime exception:" + payload);
    }
}
