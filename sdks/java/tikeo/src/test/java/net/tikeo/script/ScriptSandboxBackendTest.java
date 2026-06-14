package net.tikeo.script;

import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;

class ScriptSandboxBackendTest {
    @Test
    void parsesAutoAndExplicitSandboxBackends() {
        Assertions.assertEquals(ScriptSandboxBackend.AUTO, ScriptSandboxBackend.fromValue(null));
        Assertions.assertEquals(ScriptSandboxBackend.AUTO, ScriptSandboxBackend.fromValue("auto"));
        Assertions.assertEquals(ScriptSandboxBackend.WASMTIME, ScriptSandboxBackend.fromValue("wasmtime"));
        Assertions.assertEquals(ScriptSandboxBackend.WASMEDGE, ScriptSandboxBackend.fromValue("wasmedge"));
        Assertions.assertEquals(ScriptSandboxBackend.SRT, ScriptSandboxBackend.fromValue("srt"));
        Assertions.assertEquals(ScriptSandboxBackend.DENO, ScriptSandboxBackend.fromValue("deno"));
        Assertions.assertEquals(ScriptSandboxBackend.V8, ScriptSandboxBackend.fromValue("v8"));
    }

    @Test
    void autoSelectsDenoForJsTsAndSrtForNativeScripts() {
        Assertions.assertEquals(ScriptSandboxBackend.DENO,
                ScriptSandboxBackend.AUTO.resolve(ScriptRunnerKind.JS));
        Assertions.assertEquals(ScriptSandboxBackend.DENO,
                ScriptSandboxBackend.AUTO.resolve(ScriptRunnerKind.TS));
        Assertions.assertEquals(ScriptSandboxBackend.SRT,
                ScriptSandboxBackend.AUTO.resolve(ScriptRunnerKind.SHELL));
        Assertions.assertEquals(ScriptSandboxBackend.SRT,
                ScriptSandboxBackend.AUTO.resolve(ScriptRunnerKind.PYTHON));
        Assertions.assertEquals(ScriptSandboxBackend.SRT,
                ScriptSandboxBackend.AUTO.resolve(ScriptRunnerKind.RHAI));
        Assertions.assertEquals(ScriptSandboxBackend.SRT,
                ScriptSandboxBackend.SRT.resolve(ScriptRunnerKind.SHELL));
    }
}
