package net.tikeo.script;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;

class ScriptRunnerSupportTest {
    @Test
    void emitsCapturedStdoutAndStderrToLogSink() throws Exception {
        List<String> logs = new ArrayList<>();
        ProcessBuilder builder = new ProcessBuilder("sh", "-c", "cat; echo err-line >&2");
        ScriptRunnerTask task = new ScriptRunnerTask(
                "script-1",
                "sv-1",
                1,
                "shell",
                "echo out-line",
                sha256("echo out-line"),
                new ScriptRunnerPolicy(1000, 1048576, 1048576, false,
                        List.of(), List.of(), List.of(), List.of(), List.of()));

        ScriptRunnerSupport.runProcess(builder, ScriptRunnerKind.SHELL, task,
                (level, message) -> logs.add(level + ":" + message));

        Assertions.assertTrue(logs.stream().anyMatch(log -> log.equals("info:[script] echo out-line")));
        Assertions.assertTrue(logs.stream().anyMatch(log -> log.equals("error:[script] err-line")));
    }

    private static String sha256(String content) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256")
                .digest(content.getBytes(StandardCharsets.UTF_8)));
    }
}
