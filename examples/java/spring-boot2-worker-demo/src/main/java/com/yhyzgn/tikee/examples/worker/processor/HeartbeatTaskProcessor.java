package com.yhyzgn.tikee.examples.worker.processor;

import com.yhyzgn.tikee.processor.TaskContext;
import com.yhyzgn.tikee.processor.TaskOutcome;
import com.yhyzgn.tikee.processor.TikeeProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Fixed-rate scheduled task example. */
@Slf4j
@Component
public final class HeartbeatTaskProcessor {
    @TikeeProcessor("demo.heartbeat")
    public TaskOutcome heartbeat(TaskContext context) {
        log.info("[demo.heartbeat] tick jobId={} instanceId={}", context.jobId(), context.instanceId());
        TaskOutcome outcome = new TaskOutcome(true, "heartbeat:" + context.jobId());
        log.info("[demo.heartbeat] completed instanceId={} message='{}'", context.instanceId(), outcome.message());
        return outcome;
    }
}
