package net.tikeo.management.model;

/** Structured failure retry policy. maxAttempts includes the initial execution. */
public record JobRetryPolicy(
        boolean enabled,
        int maxAttempts,
        long initialDelaySeconds,
        int backoffMultiplier,
        long maxDelaySeconds) {
    public static JobRetryPolicy defaults() {
        return new JobRetryPolicy(true, 3, 5, 2, 60);
    }
}
