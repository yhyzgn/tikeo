/**
 * Script runner contracts and sandbox backends for Java workers.
 *
 * <p>Script jobs are dispatched to workers that advertise structured script runner capabilities.
 * The default auto strategy aligns with other SDKs: SRT for native scripts and Deno for JavaScript
 * or TypeScript, with Wasmtime available for WASM-oriented workloads.
 *
 * <p><strong>Operational cautions:</strong> production script execution must run inside a sandbox. Do not
 * register local host subprocess runners as SRT, Deno, Wasmtime, Docker, or Podman unless that real
 * boundary is configured and executable.
 */
package net.tikeo.script;
