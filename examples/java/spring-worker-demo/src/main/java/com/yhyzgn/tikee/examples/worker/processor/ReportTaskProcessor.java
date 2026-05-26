package com.yhyzgn.tikee.examples.worker.processor;

import com.yhyzgn.tikee.processor.TaskContext;
import com.yhyzgn.tikee.processor.TaskOutcome;
import com.yhyzgn.tikee.processor.TikeeProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Cron scheduled report task example. */
@Slf4j
@Component
public final class ReportTaskProcessor {
    @TikeeProcessor("demo.report")
    public TaskOutcome report(TaskContext context) {
        log.info("[demo.report] generating report jobId={} instanceId={}", context.jobId(), context.instanceId());
        TaskOutcome outcome = new TaskOutcome(true, "report:" + context.processorName());
        log.info("[demo.report] completed instanceId={} message='{}'", context.instanceId(), outcome.message());
        return outcome;
    }
}
