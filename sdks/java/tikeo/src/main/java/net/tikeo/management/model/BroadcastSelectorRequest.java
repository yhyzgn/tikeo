package net.tikeo.management.model;

import com.fasterxml.jackson.annotation.JsonInclude;
import java.util.List;
import java.util.Map;

/**
 * Optional worker selector for broadcast API triggers.
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record BroadcastSelectorRequest(
        List<String> tags,
        String region,
        String cluster,
        Map<String, String> labels) {}
