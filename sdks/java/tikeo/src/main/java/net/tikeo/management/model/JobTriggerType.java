package net.tikeo.management.model;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Job instance trigger source; API means explicit API/SDK/UI trigger, not an HTTP-calling task.
 */
public enum JobTriggerType {
    /**
 * Explicit API/SDK/UI trigger; this is not an HTTP-calling task type.
 */
    API("api"),
    /**
 * Cron scheduler trigger.
 */
    CRON("cron"),
    /**
 * Fixed-rate scheduler trigger.
 */
    FIXED_RATE("fixed_rate"),
    /**
 * Manual operator trigger.
 */
    MANUAL("manual"),
    /**
 * Workflow shard trigger.
 */
    WORKFLOW_SHARD("workflow_shard");

    private final String value;

    JobTriggerType(String value) {
        this.value = value;
    }

    @JsonValue
    public String value() {
        return value;
    }
}
