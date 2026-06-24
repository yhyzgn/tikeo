package net.tikeo.examples.worker.processor;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TikeoProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Workflow node task example. */
@Slf4j
@Component
public final class WorkflowStepTaskProcessor {
    @TikeoProcessor(value = "demo.workflow.step", description = "工作流步骤节点示例处理器")
    public TaskOutcome workflowStep(TaskContext context) {
        log.info("[demo.workflow.step] running workflow step jobId={} instanceId={}",
                context.jobId(), context.instanceId());
        TaskOutcome outcome = new TaskOutcome(true, "workflow-step:" + context.instanceId());
        log.info("[demo.workflow.step] completed instanceId={} message='{}'", context.instanceId(), outcome.message());
        return outcome;
    }
}
