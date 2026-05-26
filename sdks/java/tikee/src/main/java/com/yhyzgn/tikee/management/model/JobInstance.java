package com.yhyzgn.tikee.management.model;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;

/** Job instance returned after a trigger call. */
@JsonIgnoreProperties(ignoreUnknown = true)
public record JobInstance(
        String id,
        String jobId,
        String status,
        String triggerType,
        String executionMode,
        String createdAt,
        String updatedAt) {}
