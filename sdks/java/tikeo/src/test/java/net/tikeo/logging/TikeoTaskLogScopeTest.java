package net.tikeo.logging;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.List;
import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskLogger;
import org.junit.jupiter.api.Test;
import org.slf4j.MDC;

class TikeoTaskLogScopeTest {
    @Test
    void capturesThreadLocalTaskLogsOnlyInsideScopeAndRestoresMdc() {
        List<String> logs = new ArrayList<>();
        TaskLogger sink = (level, message) -> logs.add(level + ":" + message);

        TikeoTaskLogScope.capture("job-1", "demo.echo", "instance-1", sink, () -> {
            assertEquals("job-1", MDC.get(TikeoTaskLogScope.MDC_JOB_ID));
            assertEquals("demo.echo", MDC.get(TikeoTaskLogScope.MDC_PROCESSOR_NAME));
            assertEquals("instance-1", MDC.get(TikeoTaskLogScope.MDC_INSTANCE_ID));

            assertTrue(TikeoTaskLogScope.emit("info", "from logback"));
        });

        assertEquals(List.of("info:from logback"), logs);
        assertTrue(!TikeoTaskLogScope.emit("info", "outside task"));
        assertEquals(List.of("info:from logback"), logs);
        assertEquals(null, MDC.get(TikeoTaskLogScope.MDC_JOB_ID));
        assertEquals(null, MDC.get(TikeoTaskLogScope.MDC_PROCESSOR_NAME));
        assertEquals(null, MDC.get(TikeoTaskLogScope.MDC_INSTANCE_ID));
    }

    @Test
    void taskContextLogInfoAndLogErrorKeepDirectFallback() {
        List<String> logs = new ArrayList<>();
        TaskContext context = new TaskContext(
                "job-1",
                "demo.echo",
                "instance-1",
                new byte[0],
                (level, message) -> logs.add(level + ":" + message));

        context.logInfo("manual info");
        context.logError("manual error");

        assertEquals(List.of("info:manual info", "error:manual error"), logs);
    }
}
