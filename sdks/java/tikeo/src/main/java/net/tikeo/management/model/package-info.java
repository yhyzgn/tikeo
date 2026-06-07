/**
 * Typed management API request and response models.
 *
 * <p>Records in this package mirror the public tikeo HTTP API fields and keep SDK callers away from
 * raw JSON maps. Optional fields are omitted during serialization so updates can be partial.
 *
 * <p><strong>Operational cautions:</strong> {@code api} schedule and trigger types mean explicit
 * UI/API/SDK initiation. They do not mean the task performs an HTTP call.
 */
package net.tikeo.management.model;
