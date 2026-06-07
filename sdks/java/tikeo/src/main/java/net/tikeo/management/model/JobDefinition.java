package net.tikeo.management.model;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;

/**
 * Job definition returned by tikeo management APIs.
 */
@JsonIgnoreProperties(ignoreUnknown = true)
public record JobDefinition(
        String id,
        String namespace,
        String app,
        String name,
        String scheduleType,
        String scheduleExpr,
        String processorType,
        String processorName,
        String scriptId,
        boolean enabled,
        JobRetryPolicy retryPolicy) {}
