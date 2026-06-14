package net.tikeo.script;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import net.tikeo.processor.TaskOutcome;
import org.junit.jupiter.api.Assertions;
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

        List<String> command = runner.command(task("echo hello", policy(List.of("TIKEO_DEV_MESSAGE"))),
                tempDir.resolve("runner.wasm"));

        Assertions.assertTrue(command.contains("wasmtime"));
        Assertions.assertTrue(command.contains("run"));
        Assertions.assertTrue(command.contains("--disable-cache"));
        Assertions.assertTrue(command.contains("--env"));
        Assertions.assertTrue(command.contains("TIKEO_SCRIPT_ID=script-1"));
        Assertions.assertTrue(command.contains("TIKEO_SCRIPT_VERSION_ID=sv-1"));
        Assertions.assertTrue(command.contains("TIKEO_SCRIPT_VERSION_NUMBER=1"));
        Assertions.assertEquals(tempDir.resolve("runner.wasm").toString(), command.get(command.size() - 1));
    }

    @Test
    void executesShellScriptThroughWasmtimeRuntime() throws Exception {
        Path fakeWasmtime = tempDir.resolve("wasmtime");
        Files.writeString(fakeWasmtime, "#!/usr/bin/env sh\ncat >/tmp/tikeo-wasm-script-input\necho wasm-sandbox:$(cat /tmp/tikeo-wasm-script-input)\n");
        fakeWasmtime.toFile().setExecutable(true);
        WasmScriptRunner runner = new WasmScriptRunner(ScriptRunnerKind.SHELL, fakeWasmtime.toString(), List.of());
        List<String> logs = new ArrayList<>();

        TaskOutcome outcome = runner.run(task("echo hello", policy(List.of())),
                (level, message) -> logs.add(level + ":" + message));

        Assertions.assertTrue(outcome.success());
        Assertions.assertTrue(logs.stream().anyMatch(log -> log.equals("info:[script] wasm-sandbox:echo hello")));
    }

    @Test
    void rejectsHostGrantsThatWasmBackendDoesNotExpose() throws Exception {
        WasmScriptRunner runner = new WasmScriptRunner(ScriptRunnerKind.SHELL, "wasmtime", List.of());

        Assertions.assertThrows(ScriptRunnerException.class,
                () -> runner.command(task("echo hello", policy(true, List.of(), List.of(), List.of())),
                        tempDir.resolve("runner.wasm")));
        Assertions.assertThrows(ScriptRunnerException.class,
                () -> runner.command(task("echo hello", policy(false, List.of("secret:db"), List.of(), List.of())),
                        tempDir.resolve("runner.wasm")));
        Assertions.assertThrows(ScriptRunnerException.class,
                () -> runner.command(task("echo hello", policy(false, List.of(), List.of(), List.of("/data"))),
                        tempDir.resolve("runner.wasm")));
    }

    private static ScriptRunnerTask task(String content, ScriptRunnerPolicy policy) throws Exception {
        return new ScriptRunnerTask(
                "script-1",
                "sv-1",
                1,
                "shell",
                content,
                sha256(content),
                policy,
                ScriptSandboxBackend.WASMTIME);
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
