package net.tikeo.logging;

import java.util.Locale;
import java.util.Objects;
import net.tikeo.processor.TaskLogger;
import org.slf4j.MDC;

/**
 * Thread-local task logging scope used by native logging framework bridges.
 *
 * <p>The scope is intentionally explicit and thread-local: logs emitted outside
 * task processing, or from unrelated threads, are not forwarded to Tikeo task
 * logs. {@link net.tikeo.processor.TaskContext#logInfo(String)} and
 * {@link net.tikeo.processor.TaskContext#logError(String)} remain the direct
 * fallback path for code that does not use a logging framework bridge.</p>
 */
public final class TikeoTaskLogScope {
    public static final String MDC_JOB_ID = "tikeo.jobId";
    public static final String MDC_PROCESSOR_NAME = "tikeo.processorName";
    public static final String MDC_INSTANCE_ID = "tikeo.instanceId";

    private static final ThreadLocal<Scope> CURRENT = new ThreadLocal<>();

    private TikeoTaskLogScope() {}

    public static void capture(
            String jobId,
            String processorName,
            String instanceId,
            TaskLogger sink,
            Runnable action) {
        try {
            captureThrowing(jobId, processorName, instanceId, sink, () -> {
                action.run();
                return null;
            });
        } catch (RuntimeException error) {
            throw error;
        } catch (Exception error) {
            throw new IllegalStateException("Unexpected checked exception from Runnable task log scope", error);
        }
    }

    public static <T> T captureThrowing(
            String jobId,
            String processorName,
            String instanceId,
            TaskLogger sink,
            ThrowingSupplier<T> action) throws Exception {
        Objects.requireNonNull(action, "action");
        Scope previous = CURRENT.get();
        String previousJobId = MDC.get(MDC_JOB_ID);
        String previousProcessorName = MDC.get(MDC_PROCESSOR_NAME);
        String previousInstanceId = MDC.get(MDC_INSTANCE_ID);

        CURRENT.set(new Scope(
                Objects.requireNonNull(jobId, "jobId"),
                Objects.requireNonNull(processorName, "processorName"),
                Objects.requireNonNull(instanceId, "instanceId"),
                sink == null ? TaskLogger.NOOP : sink));
        MDC.put(MDC_JOB_ID, jobId);
        MDC.put(MDC_PROCESSOR_NAME, processorName);
        MDC.put(MDC_INSTANCE_ID, instanceId);
        try {
            return action.get();
        } finally {
            restore(MDC_JOB_ID, previousJobId);
            restore(MDC_PROCESSOR_NAME, previousProcessorName);
            restore(MDC_INSTANCE_ID, previousInstanceId);
            if (previous == null) {
                CURRENT.remove();
            } else {
                CURRENT.set(previous);
            }
        }
    }

    public static boolean emit(String level, String message) {
        Scope scope = CURRENT.get();
        if (scope == null) {
            return false;
        }
        scope.sink().log(normalizeLevel(level), message == null ? "" : message);
        return true;
    }

    public static boolean active() {
        return CURRENT.get() != null;
    }

    private static String normalizeLevel(String level) {
        if (level == null || level.isBlank()) {
            return "info";
        }
        String normalized = level.toLowerCase(Locale.ROOT);
        return normalized.equals("error") ? "error" : "info";
    }

    private static void restore(String key, String value) {
        if (value == null) {
            MDC.remove(key);
        } else {
            MDC.put(key, value);
        }
    }

    private record Scope(String jobId, String processorName, String instanceId, TaskLogger sink) {}

    @FunctionalInterface
    public interface ThrowingSupplier<T> {
        T get() throws Exception;
    }
}
