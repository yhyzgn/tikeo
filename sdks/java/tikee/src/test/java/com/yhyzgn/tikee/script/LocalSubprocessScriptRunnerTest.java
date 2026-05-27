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
                () -> runner.command(task(ScriptRunnerKind.SHELL, "shell", "echo hello", policy(), ScriptSandboxBackend.WASMTIME)));

        assertTrue(error.getMessage().contains("wasmtime"));
    }

    @Test
    void buildsDefaultCommandsForAllDevelopmentLanguages() throws Exception {
        assertEquals(List.of("sh", "-s"),
                new LocalSubprocessScriptRunner(ScriptRunnerKind.SHELL)
                        .command(task(ScriptRunnerKind.SHELL, "shell", "echo hello", policy())));
        assertEquals(List.of("python3", "-"),
                new LocalSubprocessScriptRunner(ScriptRunnerKind.PYTHON)
                        .command(task(ScriptRunnerKind.PYTHON, "python", "print('hello')", policy())));
        assertEquals(List.of("deno", "run", "--no-prompt", "-"),
                new LocalSubprocessScriptRunner(ScriptRunnerKind.JS)
                        .command(task(ScriptRunnerKind.JS, "javascript", "console.log('hello')", policy())));
        assertEquals(List.of("deno", "run", "--no-prompt", "-"),
                new LocalSubprocessScriptRunner(ScriptRunnerKind.TS)
                        .command(task(ScriptRunnerKind.TS, "typescript", "console.log('hello')", policy())));
        assertEquals(List.of("pwsh", "-NoProfile", "-NonInteractive", "-Command", "-"),
                new LocalSubprocessScriptRunner(ScriptRunnerKind.POWERSHELL)
                        .command(task(ScriptRunnerKind.POWERSHELL, "powershell", "Write-Output hello", policy())));
        assertEquals(List.of("rhai"),
                new LocalSubprocessScriptRunner(ScriptRunnerKind.RHAI)
                        .command(task(ScriptRunnerKind.RHAI, "rhai", "print(\"hello\");", policy())));
    }

    private static ScriptRunnerTask task(String content, ScriptRunnerPolicy policy) throws Exception {
        return task(ScriptRunnerKind.SHELL, "shell", content, policy, ScriptSandboxBackend.AUTO);
    }

    private static ScriptRunnerTask task(
            ScriptRunnerKind kind,
            String language,
            String content,
            ScriptRunnerPolicy policy) throws Exception {
        return task(kind, language, content, policy, ScriptSandboxBackend.AUTO);
    }

    private static ScriptRunnerTask task(
            ScriptRunnerKind kind,
            String language,
            String content,
            ScriptRunnerPolicy policy,
            ScriptSandboxBackend backend) throws Exception {
        if (ScriptRunnerKind.fromLanguage(language).orElseThrow() != kind) {
            throw new IllegalArgumentException("test language does not match runner kind");
        }
        return new ScriptRunnerTask("script-1", "sv-1", 1, language, content, sha256(content), policy, backend);
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
