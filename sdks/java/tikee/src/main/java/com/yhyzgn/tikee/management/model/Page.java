package com.yhyzgn.tikee.management.model;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import java.util.List;

/** Management API page envelope data. */
@JsonIgnoreProperties(ignoreUnknown = true)
public record Page<T>(List<T> items, String nextPageToken) {}
