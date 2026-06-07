/**
 * Worker registration metadata, structured capabilities, and cluster election declarations.
 *
 * <p>Workers connect outbound to tikeo and receive an authoritative server-assigned worker id.
 * Client instance ids are stable hints used for reconnect correlation and operational visibility.
 *
 * <p><strong>Usage:</strong> declare SDK processors, script runners, plugin processors, tags, and worker
 * cluster election fields through {@link net.tikeo.worker.WorkerCapabilitySet} and
 * {@link net.tikeo.worker.WorkerRegistration}.
 *
 * <p><strong>Operational cautions:</strong> keep capability declarations structured. Do not introduce
 * convention-over-configuration strings such as {@code plugin-processor:type}; those are not reliable
 * dispatch contracts and must remain operator metadata only.
 */
package net.tikeo.worker;
