package net.tikeo.examples.worker;

import ch.qos.logback.classic.Level;
import ch.qos.logback.classic.Logger;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import net.tikeo.examples.worker.processor.BytesTaskProcessor;
import net.tikeo.examples.worker.processor.ContextTaskProcessor;
import net.tikeo.examples.worker.processor.EchoTaskProcessor;
import net.tikeo.examples.worker.processor.FailingTaskProcessor;
import net.tikeo.examples.worker.processor.HeartbeatTaskProcessor;
import net.tikeo.examples.worker.processor.ReportTaskProcessor;
import net.tikeo.examples.worker.processor.SqlPluginTaskProcessor;
import net.tikeo.examples.worker.processor.WorkflowStepTaskProcessor;
import net.tikeo.logging.TikeoTaskLogScope;
import net.tikeo.logging.TikeoTaskLogbackAppender;
import net.tikeo.processor.TaskContext;
import org.assertj.core.api.Assertions;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.slf4j.LoggerFactory;

class EchoProcessorTest {
    private final Logger rootLogger = (Logger) LoggerFactory.getLogger("ROOT");
    private TikeoTaskLogbackAppender appender;

    @BeforeEach
    void attachTaskLogAppender() {
        appender = new TikeoTaskLogbackAppender();
        appender.setContext(rootLogger.getLoggerContext());
        appender.start();
        rootLogger.addAppender(appender);
        rootLogger.setLevel(Level.INFO);
    }

    @AfterEach
    void detachTaskLogAppender() {
        rootLogger.detachAppender(appender);
        appender.stop();
    }

    @Test
    void echoTaskHandlesApiPayload() throws Exception {
        List<String> taskLogs = new ArrayList<>();
        String echo = captured("demo.echo", "hello", taskLogs, context -> new EchoTaskProcessor().echo(context, "hello"));
        Assertions.assertThat(echo).isEqualTo("echo:hello");
        Assertions.assertThat(taskLogs).contains(
                "info:[demo.echo] received payload='hello'",
                "info:[demo.echo] completed result='echo:hello'");
    }

    @Test
    void contextTaskHandlesBroadcastStyleDispatch() throws Exception {
        var outcome = captured("demo.context", "payload", new ArrayList<>(), context -> new ContextTaskProcessor().context(context));

        Assertions.assertThat(outcome.success()).isTrue();
        Assertions.assertThat(outcome.message()).isEqualTo("context:demo.context:instance-1");
    }

    @Test
    void bytesTaskHandlesRawPayload() throws Exception {
        List<String> taskLogs = new ArrayList<>();
        String bytes = captured("demo.bytes", "abc", taskLogs,
                context -> new BytesTaskProcessor().bytes(context, "abc".getBytes(StandardCharsets.UTF_8)));
        Assertions.assertThat(bytes).isEqualTo("bytes:abc");
        Assertions.assertThat(taskLogs).contains(
                "info:[demo.bytes] received bytes payload='abc' length=3",
                "info:[demo.bytes] completed result='bytes:abc'");
    }

    @Test
    void scheduledAndWorkflowTasksReturnSuccessOutcomes() throws Exception {
        List<String> heartbeatLogs = new ArrayList<>();
        Assertions.assertThat(captured("demo.heartbeat", "", heartbeatLogs, context -> new HeartbeatTaskProcessor().heartbeat(context)).success()).isTrue();
        Assertions.assertThat(heartbeatLogs).contains(
                "info:[demo.heartbeat] tick jobId=job-1 instanceId=instance-1",
                "info:[demo.heartbeat] completed instanceId=instance-1 message='heartbeat:job-1'");

        List<String> reportLogs = new ArrayList<>();
        Assertions.assertThat(captured("demo.report", "", reportLogs, context -> new ReportTaskProcessor().report(context)).message()).isEqualTo("report:demo.report");
        Assertions.assertThat(reportLogs).contains(
                "info:[demo.report] generating report jobId=job-1 instanceId=instance-1",
                "info:[demo.report] completed instanceId=instance-1 message='report:demo.report'");

        List<String> workflowLogs = new ArrayList<>();
        Assertions.assertThat(captured("demo.workflow.step", "", workflowLogs, context -> new WorkflowStepTaskProcessor().workflowStep(context)).message())
                .startsWith("workflow-step:");
        Assertions.assertThat(workflowLogs).contains(
                "info:[demo.workflow.step] running workflow step jobId=job-1 instanceId=instance-1",
                "info:[demo.workflow.step] completed instanceId=instance-1 message='workflow-step:instance-1'");
    }

    @Test
    void failingTaskReturnsFailureOutcome() throws Exception {
        List<String> taskLogs = new ArrayList<>();
        var outcome = captured("demo.fail", "bad-input", taskLogs,
                context -> new FailingTaskProcessor().fail(context, "bad-input"));

        Assertions.assertThat(outcome.success()).isFalse();
        Assertions.assertThat(outcome.message()).isEqualTo("demo failure:bad-input");
        Assertions.assertThat(taskLogs).contains("error:[demo.fail] received payload='bad-input'");
    }

    @Test
    void exceptionTaskThrowsRuntimeException() {
        Assertions.assertThatThrownBy(() ->
                captured("demo.exception", "bad-input", new ArrayList<>(),
                        context -> new FailingTaskProcessor().exception(context, "bad-input")))
                .isInstanceOf(IllegalStateException.class)
                .hasMessageContaining("java demo runtime exception:bad-input");
    }

    @Test
    void sqlPluginProcessorLogsAndReturnsPluginOutcome() throws Exception {
        String payload = "{\"tenant\":\"billing\",\"batch\":\"2026-05-28T14:00:00+08:00\"}";

        List<String> taskLogs = new ArrayList<>();
        String result = captured("billing.sql-sync", payload, taskLogs,
                context -> new SqlPluginTaskProcessor().run(context, payload));

        Assertions.assertThat(result).isEqualTo("sql-plugin-ok:" + payload);
        Assertions.assertThat(taskLogs).contains("info:[billing.sql-sync] plugin SQL processor received payload='" + payload + "'");
    }

    private static <T> T captured(String processorName, String payload, List<String> taskLogs, ProcessorCall<T> call) throws Exception {
        TaskContext context = context(processorName, payload, taskLogs);
        return TikeoTaskLogScope.captureThrowing(
                context.jobId(),
                context.processorName(),
                context.instanceId(),
                context.logger(),
                () -> call.invoke(context));
    }

    private static TaskContext context(String processorName, String payload, List<String> taskLogs) {
        return new TaskContext(
                "job-1",
                processorName,
                "instance-1",
                payload.getBytes(StandardCharsets.UTF_8),
                (level, message) -> taskLogs.add(level + ":" + message));
    }

    @FunctionalInterface
    private interface ProcessorCall<T> {
        T invoke(TaskContext context) throws Exception;
    }
}
