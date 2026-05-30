package com.yhyzgn.tikee.worker;

import java.util.Objects;

/**
 * Structured worker-cluster master election declaration.
 */
public record WorkerClusterElection(boolean enabled, String domain, int priority) {
    public WorkerClusterElection {
        domain = Objects.requireNonNullElse(domain, "").trim();
        if (priority < 0) {
            throw new IllegalArgumentException("priority must be non-negative");
        }
    }

    public static WorkerClusterElection enabledByDefault() {
        return new WorkerClusterElection(true, "", 100);
    }

    public static WorkerClusterElection disabledElection() {
        return new WorkerClusterElection(false, "", 0);
    }

    public static WorkerClusterElection domain(String domain, int priority) {
        return new WorkerClusterElection(true, domain, priority);
    }
}
