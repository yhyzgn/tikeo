package com.yhyzgn.tikee.wasm;

import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;

class CliWasmtimeRunnerTest {
    @Test
    void commandUsesWasmtimeSandboxWithoutContainerRuntime() throws Exception {
        CliWasmtimeRunner runner = new CliWasmtimeRunner("wasmtime", List.of("--disable-cache"));
        WasmRunnerTask task = task(false);

        List<String> command = runner.command(task, Path.of("/tmp/module.wasm"));

        assertTrue(command.contains("wasmtime"));
        assertTrue(command.contains("run"));
        assertTrue(command.contains("--disable-cache"));
        assertTrue(command.contains("/tmp/module.wasm"));
    }

    @Test
    void rejectsNetworkPolicyByDefault() throws Exception {
        CliWasmtimeRunner runner = new CliWasmtimeRunner("wasmtime", List.of());
        WasmRunnerTask task = task(true);

        assertThrows(WasmRunnerException.class, () -> runner.run(task));
    }

    @Test
    void rejectsDigestMismatch() throws Exception {
        CliWasmtimeRunner runner = new CliWasmtimeRunner("wasmtime", List.of());
        byte[] module = "wasm-module".getBytes(java.nio.charset.StandardCharsets.UTF_8);
        WasmRunnerTask task = new WasmRunnerTask(
                "script_wasm",
                "sv_wasm",
                1,
                module,
                "bad-digest",
                "wasmtime",
                "_start",
                new WasmRunnerPolicy(1000, 1048576, 100000, false, List.of()));

        assertThrows(WasmRunnerException.class, () -> runner.run(task));
    }

    private static WasmRunnerTask task(boolean allowNetwork) throws Exception {
        byte[] module = "wasm-module".getBytes(java.nio.charset.StandardCharsets.UTF_8);
        return new WasmRunnerTask(
                "script_wasm",
                "sv_wasm",
                1,
                module,
                sha256(module),
                "wasmtime",
                "_start",
                new WasmRunnerPolicy(1000, 1048576, 100000, allowNetwork, List.of()));
    }

    private static String sha256(byte[] bytes) throws Exception {
        return java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(bytes));
    }
}
