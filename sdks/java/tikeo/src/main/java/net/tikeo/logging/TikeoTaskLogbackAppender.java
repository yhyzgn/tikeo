package net.tikeo.logging;

import ch.qos.logback.classic.Level;
import ch.qos.logback.classic.spi.ILoggingEvent;
import ch.qos.logback.core.AppenderBase;
import java.io.StringWriter;

/**
 * Logback appender that forwards ordinary task-scope logger calls to Tikeo task logs.
 *
 * <p>Attach this appender to application loggers/root in Logback configuration.
 * It only emits while {@link TikeoTaskLogScope} is active for the current task
 * processing thread, so application startup and unrelated request logs are left
 * untouched.</p>
 */
public final class TikeoTaskLogbackAppender extends AppenderBase<ILoggingEvent> {
    @Override
    protected void append(ILoggingEvent event) {
        if (event == null || !TikeoTaskLogScope.active() || !isTaskLogLevel(event.getLevel())) {
            return;
        }
        TikeoTaskLogScope.emit(level(event.getLevel()), message(event));
    }

    private static boolean isTaskLogLevel(Level level) {
        return Level.INFO.equals(level) || Level.ERROR.equals(level);
    }

    private static String level(Level level) {
        return Level.ERROR.equals(level) ? "error" : "info";
    }

    private static String message(ILoggingEvent event) {
        String formatted = event.getFormattedMessage();
        if (event.getThrowableProxy() == null) {
            return formatted == null ? "" : formatted;
        }
        StringWriter writer = new StringWriter();
        if (formatted != null && !formatted.isBlank()) {
            writer.write(formatted);
            writer.write(System.lineSeparator());
        }
        writeThrowable(event.getThrowableProxy(), writer, "");
        return writer.toString();
    }

    private static void writeThrowable(ch.qos.logback.classic.spi.IThrowableProxy proxy, StringWriter writer, String prefix) {
        writer.write(prefix);
        writer.write(proxy.getClassName());
        if (proxy.getMessage() != null) {
            writer.write(": ");
            writer.write(proxy.getMessage());
        }
        writer.write(System.lineSeparator());
        for (StackTraceElementProxyCompat element : StackTraceElementProxyCompat.from(proxy)) {
            writer.write("\tat ");
            writer.write(element.toString());
            writer.write(System.lineSeparator());
        }
        if (proxy.getCause() != null) {
            writer.write("Caused by: ");
            writeThrowable(proxy.getCause(), writer, "");
        }
    }

    private record StackTraceElementProxyCompat(String value) {
        static StackTraceElementProxyCompat[] from(ch.qos.logback.classic.spi.IThrowableProxy proxy) {
            ch.qos.logback.classic.spi.StackTraceElementProxy[] stackTrace = proxy.getStackTraceElementProxyArray();
            if (stackTrace == null) {
                return new StackTraceElementProxyCompat[0];
            }
            StackTraceElementProxyCompat[] result = new StackTraceElementProxyCompat[stackTrace.length];
            for (int i = 0; i < stackTrace.length; i++) {
                result[i] = new StackTraceElementProxyCompat(String.valueOf(stackTrace[i]));
            }
            return result;
        }
    }
}
