package net.tikeo.examples.worker.processor;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TikeoProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Cron scheduled report task example. */
@Slf4j
@Component
public final class ReportTaskProcessor {
    @TikeoProcessor(value = "demo.report", description = "生成报告类任务示例处理器")
    public TaskOutcome report(TaskContext context) {
        log.info("[demo.report] generating report jobId={} instanceId={}", context.jobId(), context.instanceId());
        TaskOutcome outcome = new TaskOutcome(true, "report:" + context.processorName());
        log.info("[demo.report] completed instanceId={} message='{}'", context.instanceId(), outcome.message());
        return outcome;
    }
}
