package net.tikeo.examples.worker;

import static org.assertj.core.api.Assertions.assertThat;

import net.tikeo.examples.worker.processor.BytesTaskProcessor;
import net.tikeo.examples.worker.processor.ContextTaskProcessor;
import net.tikeo.examples.worker.processor.EchoTaskProcessor;
import net.tikeo.examples.worker.processor.FailingTaskProcessor;
import net.tikeo.examples.worker.processor.HeartbeatTaskProcessor;
import net.tikeo.examples.worker.processor.ReportTaskProcessor;
import net.tikeo.examples.worker.processor.SqlPluginTaskProcessor;
import net.tikeo.examples.worker.processor.WorkflowStepTaskProcessor;
import net.tikeo.processor.TaskContext;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

class EchoProcessorTest {
    private static final Logger log = LoggerFactory.getLogger(EchoProcessorTest.class);
    @Test
    void echoTaskHandlesApiPayload() {
        List<String> taskLogs = new ArrayList<>();
        assertThat(new EchoTaskProcessor().echo(context("demo.echo", "hello", taskLogs), "hello")).isEqualTo("echo:hello");
        assertThat(taskLogs).contains(
                "info:[demo.echo] received payload='hello'",
                "info:[demo.echo] completed result='echo:hello'");
    }

    @Test
    void contextTaskHandlesBroadcastStyleDispatch() {
        var outcome = new ContextTaskProcessor().context(context("demo.context", "payload"));

        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("context:demo.context:instance-1");
    }

    @Test
    void bytesTaskHandlesRawPayload() {
        List<String> taskLogs = new ArrayList<>();
        assertThat(new BytesTaskProcessor().bytes(context("demo.bytes", "abc", taskLogs), "abc".getBytes(StandardCharsets.UTF_8))).isEqualTo("bytes:abc");
        assertThat(taskLogs).contains(
                "info:[demo.bytes] received bytes payload='abc' length=3",
                "info:[demo.bytes] completed result='bytes:abc'");
    }

    @Test
    void scheduledAndWorkflowTasksReturnSuccessOutcomes() {
        List<String> heartbeatLogs = new ArrayList<>();
        assertThat(new HeartbeatTaskProcessor().heartbeat(context("demo.heartbeat", "", heartbeatLogs)).success()).isTrue();
        assertThat(heartbeatLogs).contains(
                "info:[demo.heartbeat] tick jobId=job-1 instanceId=instance-1",
                "info:[demo.heartbeat] completed instanceId=instance-1 message='heartbeat:job-1'");

        List<String> reportLogs = new ArrayList<>();
        assertThat(new ReportTaskProcessor().report(context("demo.report", "", reportLogs)).message()).isEqualTo("report:demo.report");
        assertThat(reportLogs).contains(
                "info:[demo.report] generating report jobId=job-1 instanceId=instance-1",
                "info:[demo.report] completed instanceId=instance-1 message='report:demo.report'");

        List<String> workflowLogs = new ArrayList<>();
        assertThat(new WorkflowStepTaskProcessor().workflowStep(context("demo.workflow.step", "", workflowLogs)).message())
                .startsWith("workflow-step:");
        assertThat(workflowLogs).contains(
                "info:[demo.workflow.step] running workflow step jobId=job-1 instanceId=instance-1",
                "info:[demo.workflow.step] completed instanceId=instance-1 message='workflow-step:instance-1'");
    }

    @Test
    void failingTaskReturnsFailureOutcome() {
        List<String> taskLogs = new ArrayList<>();
        var outcome = new FailingTaskProcessor().fail(context("demo.fail", "bad-input", taskLogs), "bad-input");

        assertThat(outcome.success()).isFalse();
        assertThat(outcome.message()).isEqualTo("demo failure:bad-input");
        assertThat(taskLogs).contains("error:[demo.fail] received payload='bad-input'");
    }

    @Test
    void exceptionTaskThrowsRuntimeException() {
        org.assertj.core.api.Assertions.assertThatThrownBy(() ->
                new FailingTaskProcessor().exception(context("demo.exception", "bad-input"), "bad-input"))
                .isInstanceOf(IllegalStateException.class)
                .hasMessageContaining("java demo runtime exception:bad-input");
    }

    @Test
    void sqlPluginProcessorLogsAndReturnsPluginOutcome() {
        String payload = "{\"tenant\":\"billing\",\"batch\":\"2026-05-28T14:00:00+08:00\"}";
        log.info("[java-demo-plugin-test] invoking billing.sql-sync with payload={}", payload);

        List<String> taskLogs = new ArrayList<>();
        String result = new SqlPluginTaskProcessor().run(context("billing.sql-sync", payload, taskLogs), payload);

        log.info("[java-demo-plugin-test] billing.sql-sync result={}", result);
        assertThat(result).isEqualTo("sql-plugin-ok:" + payload);
        assertThat(taskLogs).contains("info:[billing.sql-sync] plugin SQL processor received payload='" + payload + "'");
    }

    private static TaskContext context(String processorName, String payload) {
        return context(processorName, payload, new ArrayList<>());
    }

    private static TaskContext context(String processorName, String payload, List<String> taskLogs) {
        return new TaskContext(
                "job-1",
                processorName,
                "instance-1",
                payload.getBytes(StandardCharsets.UTF_8),
                (level, message) -> taskLogs.add(level + ":" + message));
    }
}
