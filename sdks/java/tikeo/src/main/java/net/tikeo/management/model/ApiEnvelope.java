package net.tikeo.management.model;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;

/**
 * tikeo management API response envelope.
 */
@JsonIgnoreProperties(ignoreUnknown = true)
public record ApiEnvelope<T>(int code, String message, T data) {}
