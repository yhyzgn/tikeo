/**
 * Sandbox tool discovery and installation utilities.
 *
 * <p>The Java SDK can check and install toolchains such as SRT, Deno, Wasmtime, WasmEdge, V8-facing
 * runtimes, PowerShell, and Rhai under a managed SDK directory. This keeps worker startup repeatable
 * across bare metal, VM, and container environments.
 *
 * <p><strong>Operational cautions:</strong> automatic installers require network access and platform
 * prerequisites. For production clusters, prefer pre-baked worker images or controlled artifact
 * mirrors and keep runtime checks enabled so workers fail closed when sandbox tools are missing.
 */
package net.tikeo.sandbox;
