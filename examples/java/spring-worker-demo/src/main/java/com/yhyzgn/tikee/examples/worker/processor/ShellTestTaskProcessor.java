package com.yhyzgn.tikee.examples.worker.processor;

import com.yhyzgn.tikee.processor.TaskContext;
import com.yhyzgn.tikee.processor.TaskOutcome;
import com.yhyzgn.tikee.processor.TikeeProcessor;
import java.nio.charset.StandardCharsets;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Manual API-triggered shell-style demo task. */
@Slf4j
@Component
public final class ShellTestTaskProcessor {
    @TikeeProcessor("shell.test")
    public TaskOutcome shellTest(TaskContext context) {
        String payload = new String(context.payload(), StandardCharsets.UTF_8);
        log.info("[shell.test] received jobId={} instanceId={} processor={} payload='{}'",
                context.jobId(), context.instanceId(), context.processorName(), payload);
        String message = "shell-test:" + context.instanceId();
        log.info("[shell.test] completed message='{}'", message);
        return new TaskOutcome(true, message);
    }
}
