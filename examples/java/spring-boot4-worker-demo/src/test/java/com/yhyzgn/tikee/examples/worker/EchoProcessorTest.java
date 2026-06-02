package com.yhyzgn.tikee.examples.worker;

import static org.assertj.core.api.Assertions.assertThat;

import com.yhyzgn.tikee.examples.worker.processor.BytesTaskProcessor;
import com.yhyzgn.tikee.examples.worker.processor.ContextTaskProcessor;
import com.yhyzgn.tikee.examples.worker.processor.EchoTaskProcessor;
import com.yhyzgn.tikee.examples.worker.processor.FailingTaskProcessor;
import com.yhyzgn.tikee.examples.worker.processor.HeartbeatTaskProcessor;
import com.yhyzgn.tikee.examples.worker.processor.ReportTaskProcessor;
import com.yhyzgn.tikee.examples.worker.processor.SqlPluginTaskProcessor;
import com.yhyzgn.tikee.examples.worker.processor.WorkflowStepTaskProcessor;
import com.yhyzgn.tikee.processor.TaskContext;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

class EchoProcessorTest {
    private static final Logger log = LoggerFactory.getLogger(EchoProcessorTest.class);
    @Test
    void echoTaskHandlesApiPayload() {
        assertThat(new EchoTaskProcessor().echo("hello")).isEqualTo("echo:hello");
    }

    @Test
    void contextTaskHandlesBroadcastStyleDispatch() {
        var outcome = new ContextTaskProcessor().context(context("demo.context", "payload"));

        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("context:demo.context:instance-1");
    }

    @Test
    void bytesTaskHandlesRawPayload() {
        assertThat(new BytesTaskProcessor().bytes("abc".getBytes(StandardCharsets.UTF_8))).isEqualTo("bytes:abc");
    }

    @Test
    void scheduledAndWorkflowTasksReturnSuccessOutcomes() {
        assertThat(new HeartbeatTaskProcessor().heartbeat(context("demo.heartbeat", "")).success()).isTrue();
        assertThat(new ReportTaskProcessor().report(context("demo.report", "")).message()).isEqualTo("report:demo.report");
        assertThat(new WorkflowStepTaskProcessor().workflowStep(context("demo.workflow.step", "")).message())
                .startsWith("workflow-step:");
    }

    @Test
    void failingTaskReturnsFailureOutcome() {
        var outcome = new FailingTaskProcessor().fail("bad-input");

        assertThat(outcome.success()).isFalse();
        assertThat(outcome.message()).isEqualTo("demo failure:bad-input");
    }

    @Test
    void sqlPluginProcessorLogsAndReturnsPluginOutcome() {
        String payload = "{\"tenant\":\"billing\",\"batch\":\"2026-05-28T14:00:00+08:00\"}";
        log.info("[java-demo-plugin-test] invoking billing.sql-sync with payload={}", payload);

        String result = new SqlPluginTaskProcessor().run(payload);

        log.info("[java-demo-plugin-test] billing.sql-sync result={}", result);
        assertThat(result).isEqualTo("sql-plugin-ok:" + payload);
    }

    private static TaskContext context(String processorName, String payload) {
        return new TaskContext(
                "job-1",
                processorName,
                "instance-1",
                payload.getBytes(StandardCharsets.UTF_8));
    }
}
