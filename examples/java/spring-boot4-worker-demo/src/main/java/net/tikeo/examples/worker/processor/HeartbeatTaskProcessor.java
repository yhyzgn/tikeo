package net.tikeo.examples.worker.processor;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TikeoProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Fixed-rate scheduled task example. */
@Slf4j
@Component
public final class HeartbeatTaskProcessor {
    @TikeoProcessor(value = "demo.heartbeat", description = "心跳/运行态检查示例处理器")
    public TaskOutcome heartbeat(TaskContext context) {
        log.info("[demo.heartbeat] tick jobId={} instanceId={}", context.jobId(), context.instanceId());
        TaskOutcome outcome = new TaskOutcome(true, "heartbeat:" + context.jobId());
        log.info("[demo.heartbeat] completed instanceId={} message='{}'", context.instanceId(), outcome.message());
        return outcome;
    }
}
