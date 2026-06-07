/**
 * WASM processor execution contracts.
 *
 * <p>WASM support lets a worker execute immutable module snapshots with an explicit runtime,
 * entrypoint, digest, timeout, fuel, memory, and environment policy.
 *
 * <p><strong>Operational cautions:</strong> verify module digests before execution, keep network access
 * disabled unless policy explicitly allows it, and report failures with task-safe messages.
 */
package net.tikeo.wasm;
