package net.tikeo.wasm;

import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.List;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;

class CliWasmtimeRunnerTest {
    @Test
    void commandUsesWasmtimeSandboxWithoutContainerRuntime() throws Exception {
        CliWasmtimeRunner runner = new CliWasmtimeRunner("wasmtime", List.of("--disable-cache"));
        WasmRunnerTask task = task(false);

        List<String> command = runner.command(task, Path.of("/tmp/module.wasm"));

        Assertions.assertTrue(command.contains("wasmtime"));
        Assertions.assertTrue(command.contains("run"));
        Assertions.assertTrue(command.contains("--disable-cache"));
        Assertions.assertTrue(command.contains("/tmp/module.wasm"));
    }

    @Test
    void rejectsNetworkPolicyByDefault() throws Exception {
        CliWasmtimeRunner runner = new CliWasmtimeRunner("wasmtime", List.of());
        WasmRunnerTask task = task(true);

        Assertions.assertThrows(WasmRunnerException.class, () -> runner.run(task));
    }

    @Test
    void rejectsDigestMismatch() throws Exception {
        CliWasmtimeRunner runner = new CliWasmtimeRunner("wasmtime", List.of());
        byte[] module = "wasm-module".getBytes(StandardCharsets.UTF_8);
        WasmRunnerTask task = new WasmRunnerTask(
                "script_wasm",
                "sv_wasm",
                1,
                module,
                "bad-digest",
                "wasmtime",
                "_start",
                new WasmRunnerPolicy(1000, 1048576, 100000, false, List.of()));

        Assertions.assertThrows(WasmRunnerException.class, () -> runner.run(task));
    }

    private static WasmRunnerTask task(boolean allowNetwork) throws Exception {
        byte[] module = "wasm-module".getBytes(StandardCharsets.UTF_8);
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
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
    }
}
