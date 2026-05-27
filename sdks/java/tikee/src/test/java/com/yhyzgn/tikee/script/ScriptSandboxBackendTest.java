package com.yhyzgn.tikee.script;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class ScriptSandboxBackendTest {
    @Test
    void parsesAutoAndExplicitSandboxBackends() {
        assertEquals(ScriptSandboxBackend.AUTO, ScriptSandboxBackend.fromValue(null));
        assertEquals(ScriptSandboxBackend.AUTO, ScriptSandboxBackend.fromValue("auto"));
        assertEquals(ScriptSandboxBackend.WASMTIME, ScriptSandboxBackend.fromValue("wasmtime"));
        assertEquals(ScriptSandboxBackend.WASMEDGE, ScriptSandboxBackend.fromValue("wasmedge"));
        assertEquals(ScriptSandboxBackend.SRT, ScriptSandboxBackend.fromValue("srt"));
        assertEquals(ScriptSandboxBackend.DENO, ScriptSandboxBackend.fromValue("deno"));
        assertEquals(ScriptSandboxBackend.V8, ScriptSandboxBackend.fromValue("v8"));
    }

    @Test
    void autoSelectsDenoForJsTsAndSrtForNativeScripts() {
        assertEquals(ScriptSandboxBackend.DENO,
                ScriptSandboxBackend.AUTO.resolve(ScriptRunnerKind.JS));
        assertEquals(ScriptSandboxBackend.DENO,
                ScriptSandboxBackend.AUTO.resolve(ScriptRunnerKind.TS));
        assertEquals(ScriptSandboxBackend.SRT,
                ScriptSandboxBackend.AUTO.resolve(ScriptRunnerKind.SHELL));
        assertEquals(ScriptSandboxBackend.SRT,
                ScriptSandboxBackend.AUTO.resolve(ScriptRunnerKind.PYTHON));
        assertEquals(ScriptSandboxBackend.SRT,
                ScriptSandboxBackend.SRT.resolve(ScriptRunnerKind.SHELL));
    }
}
