package com.yhyzgn.tikee.examples.worker.processor;

import com.yhyzgn.tikee.processor.TaskContext;
import com.yhyzgn.tikee.processor.TaskOutcome;
import com.yhyzgn.tikee.processor.TikeeProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Workflow node task example. */
@Slf4j
@Component
public final class WorkflowStepTaskProcessor {
    @TikeeProcessor("demo.workflow.step")
    public TaskOutcome workflowStep(TaskContext context) {
        log.info("[demo.workflow.step] running workflow step jobId={} instanceId={}",
                context.jobId(), context.instanceId());
        context.logInfo("[demo.workflow.step] running workflow step jobId=" + context.jobId() + " instanceId=" + context.instanceId());
        TaskOutcome outcome = new TaskOutcome(true, "workflow-step:" + context.instanceId());
        log.info("[demo.workflow.step] completed instanceId={} message='{}'", context.instanceId(), outcome.message());
        context.logInfo("[demo.workflow.step] completed instanceId=" + context.instanceId() + " message='" + outcome.message() + "'");
        return outcome;
    }
}
