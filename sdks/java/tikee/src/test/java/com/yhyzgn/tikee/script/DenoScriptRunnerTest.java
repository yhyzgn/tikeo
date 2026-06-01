package com.yhyzgn.tikee.script;

import static org.junit.jupiter.api.Assertions.assertTrue;

import com.yhyzgn.tikee.processor.TaskOutcome;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class DenoScriptRunnerTest {
    @TempDir
    Path tempDir;

    @Test
    void acceptsExplicitV8BackendForJavaScriptThroughDenoEngine() throws Exception {
        Path fakeDeno = tempDir.resolve("deno");
        Files.writeString(
            fakeDeno,
            "#!/usr/bin/env sh\n" +
                "cat >/dev/null\n" +
                "echo deno-v8-ok\n"
        );
        fakeDeno.toFile().setExecutable(true);
        DenoScriptRunner runner = new DenoScriptRunner(
            ScriptRunnerKind.JS,
            fakeDeno.toString()
        );
        List<String> logs = new ArrayList<>();

        TaskOutcome outcome = runner.run(
            task("console.log('deno-v8-ok');", ScriptSandboxBackend.V8),
            (level, message) -> logs.add(level + ":" + message)
        );

        assertTrue(outcome.success(), outcome.message());
        assertTrue(
            logs.stream().anyMatch(log -> log.equals("info:[script] deno-v8-ok"))
        );
    }

    private static ScriptRunnerTask task(
        String content,
        ScriptSandboxBackend backend
    ) throws Exception {
        return new ScriptRunnerTask(
            "script-1",
            "sv-1",
            1,
            "javascript",
            content,
            ScriptRunnerSupport.sha256Hex(content),
            new ScriptRunnerPolicy(
                1000,
                1048576,
                1048576,
                false,
                List.of(),
                List.of(),
                List.of(),
                List.of(),
                List.of()
            ),
            backend
        );
    }
}
