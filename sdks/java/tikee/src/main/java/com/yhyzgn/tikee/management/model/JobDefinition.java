package com.yhyzgn.tikee.management.model;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;

/** Job definition returned by tikee management APIs. */
@JsonIgnoreProperties(ignoreUnknown = true)
public record JobDefinition(
        String id,
        String namespace,
        String app,
        String name,
        String scheduleType,
        String scheduleExpr,
        String processorName,
        String scriptId,
        boolean enabled) {}
