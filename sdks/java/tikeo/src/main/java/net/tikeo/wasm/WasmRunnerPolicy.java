package net.tikeo.wasm;

import java.util.List;

/** Resource and capability policy for WASM processor execution. */
public record WasmRunnerPolicy(
        long timeoutMillis,
        long maxMemoryBytes,
        long fuel,
        boolean allowNetwork,
        List<String> allowedEnvVars) {
    public WasmRunnerPolicy {
        allowedEnvVars = List.copyOf(allowedEnvVars == null ? List.of() : allowedEnvVars);
        if (timeoutMillis <= 0) {
            throw new WasmRunnerException("wasm timeout must be greater than zero");
        }
        if (maxMemoryBytes <= 0) {
            throw new WasmRunnerException("wasm memory limit must be greater than zero");
        }
        if (fuel <= 0) {
            throw new WasmRunnerException("wasm fuel budget must be greater than zero");
        }
    }
}
