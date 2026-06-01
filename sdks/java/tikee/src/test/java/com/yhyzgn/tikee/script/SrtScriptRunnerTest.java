package com.yhyzgn.tikee.script;

import static org.junit.jupiter.api.Assertions.assertTrue;

import com.yhyzgn.tikee.processor.TaskOutcome;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class SrtScriptRunnerTest {
    @TempDir
    Path tempDir;

    @Test
    void runsShellScriptThroughSrtCommand() throws Exception {
        Path fakeSrt = tempDir.resolve("srt");
        Files.writeString(
            fakeSrt,
            "#!/usr/bin/env sh\n" +
                "if [ \"$1\" = \"--settings\" ]; then shift 2; fi\n" +
                "if [ \"$1\" = \"-c\" ]; then shift; sh -c \"$1\"; fi\n"
        );
        fakeSrt.toFile().setExecutable(true);
        SrtScriptRunner runner = new SrtScriptRunner(
            ScriptRunnerKind.SHELL,
            fakeSrt.toString()
        );
        List<String> logs = new ArrayList<>();

        TaskOutcome outcome = runner.run(
            task("echo srt-shell-ok"),
            (level, message) -> logs.add(level + ":" + message)
        );

        assertTrue(outcome.success(), outcome.message());
        assertTrue(
            logs.stream().anyMatch(log -> log.equals("info:[script] srt-shell-ok"))
        );
    }

    private static String sha256(String content) throws Exception {
        java.security.MessageDigest digest = java.security.MessageDigest.getInstance(
            "SHA-256"
        );
        byte[] hash = digest.digest(
            content.getBytes(java.nio.charset.StandardCharsets.UTF_8)
        );
        StringBuilder builder = new StringBuilder();
        for (byte value : hash) {
            builder.append(String.format("%02x", value));
        }
        return builder.toString();
    }

    private static ScriptRunnerTask task(String content) throws Exception {
        return new ScriptRunnerTask(
            "script-1",
            "sv-1",
            1,
            "shell",
            content,
            sha256(content),
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
            ScriptSandboxBackend.AUTO
        );
    }
}
