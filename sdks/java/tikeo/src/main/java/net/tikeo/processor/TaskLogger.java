package net.tikeo.processor;

/** Emits task-scoped instance logs without capturing unrelated process output. */
@FunctionalInterface
public interface TaskLogger {
    TaskLogger NOOP = (level, message) -> {};

    void log(String level, String message);

    default void info(String message) {
        log("info", message);
    }

    default void error(String message) {
        log("error", message);
    }
}
