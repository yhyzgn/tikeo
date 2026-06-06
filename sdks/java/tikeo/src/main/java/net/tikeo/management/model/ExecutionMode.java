package net.tikeo.management.model;

import com.fasterxml.jackson.annotation.JsonValue;

/** Job execution fan-out mode. */
public enum ExecutionMode {
    SINGLE("single"),
    BROADCAST("broadcast");

    private final String value;

    ExecutionMode(String value) {
        this.value = value;
    }

    @JsonValue
    public String value() {
        return value;
    }
}
