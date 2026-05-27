package com.yhyzgn.tikee.script;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.yhyzgn.tikee.processor.TaskOutcome;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class WasmScriptRunnerTest {
    @TempDir
    Path tempDir;

    @Test
    void buildsWasmtimeCommandForBundledShellRuntime() throws Exception {
        WasmScriptRunner runner = new WasmScriptRunner(
                ScriptRunnerKind.SHELL,
                "wasmtime",
                List.of("--disable-cache"));

        List<String> command = runner.command(task("echo hello", policy(List.of("TIKEE_DEV_MESSAGE"))),
                tempDir.resolve("runner.wasm"));

        assertTrue(command.contains("wasmtime"));
        assertTrue(command.contains("run"));
        assertTrue(command.contains("--disable-cache"));
        assertTrue(command.contains("--env"));
        assertTrue(command.contains("TIKEE_SCRIPT_ID=script-1"));
        assertTrue(command.contains("TIKEE_SCRIPT_VERSION_ID=sv-1"));
        assertTrue(command.contains("TIKEE_SCRIPT_VERSION_NUMBER=1"));
        assertEquals(tempDir.resolve("runner.wasm").toString(), command.get(command.size() - 1));
    }

    @Test
    void executesShellScriptThroughWasmtimeRuntime() throws Exception {
        Path fakeWasmtime = tempDir.resolve("wasmtime");
        Files.writeString(fakeWasmtime, "#!/usr/bin/env sh\ncat >/tmp/tikee-wasm-script-input\necho wasm-sandbox:$(cat /tmp/tikee-wasm-script-input)\n");
        fakeWasmtime.toFile().setExecutable(true);
        WasmScriptRunner runner = new WasmScriptRunner(ScriptRunnerKind.SHELL, fakeWasmtime.toString(), List.of());
        List<String> logs = new ArrayList<>();

        TaskOutcome outcome = runner.run(task("echo hello", policy(List.of())),
                (level, message) -> logs.add(level + ":" + message));

        assertTrue(outcome.success());
        assertTrue(logs.stream().anyMatch(log -> log.equals("info:[script] wasm-sandbox:echo hello")));
    }

    @Test
    void rejectsHostGrantsThatWasmBackendDoesNotExpose() throws Exception {
        WasmScriptRunner runner = new WasmScriptRunner(ScriptRunnerKind.SHELL, "wasmtime", List.of());

        assertThrows(ScriptRunnerException.class,
                () -> runner.command(task("echo hello", policy(true, List.of(), List.of(), List.of())),
                        tempDir.resolve("runner.wasm")));
        assertThrows(ScriptRunnerException.class,
                () -> runner.command(task("echo hello", policy(false, List.of("secret:db"), List.of(), List.of())),
                        tempDir.resolve("runner.wasm")));
        assertThrows(ScriptRunnerException.class,
                () -> runner.command(task("echo hello", policy(false, List.of(), List.of(), List.of("/data"))),
                        tempDir.resolve("runner.wasm")));
    }

    private static ScriptRunnerTask task(String content, ScriptRunnerPolicy policy) throws Exception {
        return new ScriptRunnerTask("script-1", "sv-1", 1, "shell", content, sha256(content), policy);
    }

    private static ScriptRunnerPolicy policy(List<String> allowedEnvVars) {
        return policy(false, List.of(), allowedEnvVars, List.of());
    }

    private static ScriptRunnerPolicy policy(
            boolean allowNetwork,
            List<String> secrets,
            List<String> allowedEnvVars,
            List<String> readOnlyPaths) {
        return new ScriptRunnerPolicy(
                1000,
                1048576,
                1048576,
                allowNetwork,
                List.of(),
                allowedEnvVars,
                readOnlyPaths,
                List.of(),
                secrets);
    }

    private static String sha256(String content) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256")
                .digest(content.getBytes(StandardCharsets.UTF_8)));
    }
}
