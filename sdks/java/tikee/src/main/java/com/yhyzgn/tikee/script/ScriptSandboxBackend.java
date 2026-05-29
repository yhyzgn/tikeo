package com.yhyzgn.tikee.script;

import java.util.Locale;

/** Supported sandbox backends for dynamic script execution. */
public enum ScriptSandboxBackend {
    AUTO("auto"),
    WASMTIME("wasmtime"),
    WASMEDGE("wasmedge"),
    SRT("srt"),
    DENO("deno"),
    V8("v8"),
    DOCKER("docker"),
    PODMAN("podman"),
    CUSTOM("custom");

    private final String value;

    ScriptSandboxBackend(String value) {
        this.value = value;
    }

    public String value() {
        return value;
    }

    public ScriptSandboxBackend resolve(ScriptRunnerKind kind) {
        if (this != AUTO) {
            return this;
        }
        return switch (kind) {
            case JS, TS -> DENO;
            case SHELL, PYTHON, POWERSHELL, PHP, GROOVY, RHAI -> SRT;
            default -> WASMTIME;
        };
    }

    public static ScriptSandboxBackend fromValue(String value) {
        String normalized = value == null ? "" : value.trim().toLowerCase(Locale.ROOT);
        return switch (normalized) {
            case "", "auto" -> AUTO;
            case "wasmtime" -> WASMTIME;
            case "wasmedge", "wasm_edge", "wasm-edge" -> WASMEDGE;
            case "srt", "anthropic_srt", "anthropic-srt", "sandbox_runtime", "sandbox-runtime" -> SRT;
            case "deno" -> DENO;
            case "v8", "v8_isolate", "v8-isolate" -> V8;
            case "docker" -> DOCKER;
            case "podman" -> PODMAN;
            case "custom" -> CUSTOM;
            default -> throw new ScriptRunnerException("unsupported script sandbox backend: " + value);
        };
    }
}
