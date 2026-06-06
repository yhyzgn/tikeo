package net.tikeo.wasm;

/** Immutable WASM module snapshot passed to a sandbox runtime. */
public record WasmRunnerTask(
        String scriptId,
        String versionId,
        long versionNumber,
        byte[] module,
        String moduleSha256,
        String runtime,
        String entrypoint,
        WasmRunnerPolicy policy) {}
