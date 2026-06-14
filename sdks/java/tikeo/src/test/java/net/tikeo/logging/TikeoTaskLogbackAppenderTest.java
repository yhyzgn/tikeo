package net.tikeo.logging;

import static org.junit.jupiter.api.Assertions.assertEquals;

import ch.qos.logback.classic.Level;
import ch.qos.logback.classic.Logger;
import java.util.ArrayList;
import java.util.List;
import net.tikeo.processor.TaskLogger;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;
import org.slf4j.LoggerFactory;

class TikeoTaskLogbackAppenderTest {
    private final Logger logger = (Logger) LoggerFactory.getLogger("net.tikeo.logging.bridge-test");
    private TikeoTaskLogbackAppender appender;

    @AfterEach
    void tearDown() {
        if (appender != null) {
            logger.detachAppender(appender);
            appender.stop();
        }
    }

    @Test
    void forwardsInfoAndErrorEventsInsideTaskScopeOnly() {
        appender = new TikeoTaskLogbackAppender();
        appender.setContext(logger.getLoggerContext());
        appender.start();
        logger.addAppender(appender);
        logger.setLevel(Level.INFO);
        logger.setAdditive(false);

        List<String> logs = new ArrayList<>();
        TaskLogger sink = (level, message) -> logs.add(level + ":" + message);

        logger.info("outside should not be captured");
        TikeoTaskLogScope.capture("job-1", "demo.echo", "instance-1", sink, () -> {
            logger.info("hello {}", "task");
            logger.error("failed {}", "task", new IllegalStateException("boom"));
        });
        logger.info("outside after should not be captured");

        assertEquals(2, logs.size());
        assertEquals("info:hello task", logs.get(0));
        assertEquals(true, logs.get(1).startsWith("error:failed task"));
        assertEquals(true, logs.get(1).contains("java.lang.IllegalStateException: boom"));
    }
}
