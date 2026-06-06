package net.tikeo.processor;

import java.util.Objects;

/**
 * Result returned by a worker task processor.
 */
public record TaskOutcome(boolean success, String message) {
    public TaskOutcome {
        message = message == null ? "" : message;
    }

    public static TaskOutcome succeeded() {
        return new TaskOutcome(true, "");
    }

    public static TaskOutcome failed(String message) {
        return new TaskOutcome(false, Objects.requireNonNullElse(message, ""));
    }
}
