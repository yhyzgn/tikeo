/**
 * Stable worker client-instance identity helpers.
 *
 * <p>The helpers derive or persist client instance ids for Kubernetes Pods, containers, systemd
 * services, VMs, and bare-metal slots. The server still assigns the authoritative worker id after
 * registration.
 *
 * <p><strong>Operational cautions:</strong> choose a runtime identity that distinguishes replicas. Reusing
 * a client instance id across independent workers makes reconnect correlation ambiguous.
 */
package net.tikeo.worker.identity;
