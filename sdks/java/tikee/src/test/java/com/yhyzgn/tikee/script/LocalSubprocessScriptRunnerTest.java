package com.yhyzgn.tikee.script;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.yhyzgn.tikee.processor.TaskOutcome;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import org.junit.jupiter.api.Test;

class LocalSubprocessScriptRunnerTest {
    @Test
    void executesRealShellSyntaxForDevelopmentWorkers() throws Exception {
        LocalSubprocessScriptRunner runner = new LocalSubprocessScriptRunner(ScriptRunnerKind.SHELL);
        List<String> logs = new ArrayList<>();

        TaskOutcome outcome = runner.run(task("set -eu\necho shell-ok", policy()),
                (level, message) -> logs.add(level + ":" + message));

        assertTrue(outcome.success());
        assertTrue(logs.stream().anyMatch(log -> log.equals("info:[script] shell-ok")));
    }

    @Test
    void rejectsExplicitWasmtimeBackendForLocalShellRunner() throws Exception {
        LocalSubprocessScriptRunner runner = new LocalSubprocessScriptRunner(ScriptRunnerKind.SHELL);

        ScriptRunnerException error = assertThrows(ScriptRunnerException.class,
                () -> runner.command(task("echo hello", policy(), ScriptSandboxBackend.WASMTIME)));

        assertTrue(error.getMessage().contains("requested: wasmtime"));
    }

    @Test
    void buildsDefaultShellCommand() throws Exception {
        LocalSubprocessScriptRunner runner = new LocalSubprocessScriptRunner(ScriptRunnerKind.SHELL);

        assertEquals(List.of("sh", "-s"), runner.command(task("echo hello", policy())));
    }

    private static ScriptRunnerTask task(String content, ScriptRunnerPolicy policy) throws Exception {
        return task(content, policy, ScriptSandboxBackend.AUTO);
    }

    private static ScriptRunnerTask task(
            String content,
            ScriptRunnerPolicy policy,
            ScriptSandboxBackend backend) throws Exception {
        return new ScriptRunnerTask("script-1", "sv-1", 1, "shell", content, sha256(content), policy, backend);
    }

    private static ScriptRunnerPolicy policy() {
        return new ScriptRunnerPolicy(
                1000,
                1048576,
                1048576,
                false,
                List.of(),
                List.of(),
                List.of(),
                List.of(),
                List.of());
    }

    private static String sha256(String content) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256")
                .digest(content.getBytes(StandardCharsets.UTF_8)));
    }
}
