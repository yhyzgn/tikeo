package net.tikeo.management.model;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Job schedule source; API means explicit API/SDK/UI trigger, not an HTTP-calling task.
 */
public enum JobScheduleType {
    /**
 * Explicit API/SDK/UI trigger; this is not an HTTP-calling task type.
 */
    API("api"),
    /**
 * Cron expression trigger.
 */
    CRON("cron"),
    /**
 * Fixed-rate scheduler trigger.
 */
    FIXED_RATE("fixed_rate"),
    /**
 * Fixed-delay scheduler trigger.
 */
    FIXED_DELAY("fixed_delay");

    private final String value;

    JobScheduleType(String value) {
        this.value = value;
    }

    @JsonValue
    public String value() {
        return value;
    }
}
