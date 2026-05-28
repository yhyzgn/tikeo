package com.yhyzgn.tikee.wasm;

import com.yhyzgn.tikee.worker.WorkerCapabilitySet;
import java.util.List;
import java.util.Optional;

/** Explicit registry for the worker-side WASM sandbox runner. */
public final class WasmRunnerRegistry {
    private WasmRunner runner;

    public WasmRunnerRegistry register(WasmRunner runner) {
        this.runner = runner;
        return this;
    }

    public Optional<WasmRunner> runner() {
        return Optional.ofNullable(runner);
    }

    public List<String> capabilities() {
        return runner == null ? List.of() : List.of("script:wasm");
    }

    public List<WorkerCapabilitySet.ScriptRunner> structuredCapabilities() {
        return runner == null ? List.of() : List.of(new WorkerCapabilitySet.ScriptRunner("wasm", "wasmtime"));
    }

    public boolean isEmpty() {
        return runner == null;
    }
}
