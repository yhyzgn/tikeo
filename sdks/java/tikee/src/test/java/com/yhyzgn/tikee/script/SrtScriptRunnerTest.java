package com.yhyzgn.tikee.script;

import static org.junit.jupiter.api.Assertions.assertTrue;

import com.yhyzgn.tikee.processor.TaskOutcome;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
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
                "settings=\"\"\n" +
                "if [ \"$1\" = \"--settings\" ]; then settings=\"$2\"; shift 2; fi\n" +
                "grep -q '\"deniedDomains\"' \"$settings\" || exit 31\n" +
                "command -v rg >/dev/null 2>&1 || exit 32\n" +
                "if [ \"$1\" = \"-c\" ]; then shift; sh -c \"$1\"; fi\n"
        );
        fakeSrt.toFile().setExecutable(true);
        Path fakeRgDir = tempDir.resolve("bin");
        Files.createDirectories(fakeRgDir);
        Path fakeRg = fakeRgDir.resolve("rg");
        Files.writeString(fakeRg, "#!/usr/bin/env sh\necho ripgrep 14.0.0\n");
        fakeRg.toFile().setExecutable(true);
        SrtScriptRunner runner = new SrtScriptRunner(
            ScriptRunnerKind.SHELL,
            fakeSrt.toString(),
            "sh",
            List.of(fakeRgDir.toString())
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

    @Test
    void rhaiDiagnosticOutputFailsEvenWhenProcessExitsZero() throws Exception {
        Path fakeSrt = tempDir.resolve("srt-rhai");
        Files.writeString(
            fakeSrt,
            "#!/usr/bin/env sh\n" +
                "if [ \"$1\" = \"--settings\" ]; then shift 2; fi\n" +
                "if [ \"$1\" = \"-c\" ]; then shift; sh -c \"$1\"; fi\n"
        );
        fakeSrt.toFile().setExecutable(true);
        Path fakeRhai = tempDir.resolve("rhai-run");
        Files.writeString(
            fakeRhai,
            "#!/usr/bin/env sh\n" +
                "printf '%s\\n' \"                                                   ^ Syntax error: 'case' is a reserved keyword\"\n" +
                "printf '%s\\n' \"1: let result = #{ language: \\\"rhai\\\", status: \\\"ok\\\", case: \\\"manual-acceptance\\\" };\" >&2\n" +
                "exit 0\n"
        );
        fakeRhai.toFile().setExecutable(true);
        SrtScriptRunner runner = new SrtScriptRunner(
            ScriptRunnerKind.RHAI,
            fakeSrt.toString(),
            fakeRhai.toString(),
            List.of(tempDir.toString())
        );

        TaskOutcome outcome = runner.run(task("rhai", "print(\"ok\");"));

        assertTrue(!outcome.success());
        assertTrue(outcome.message().contains("Syntax error"));
        assertTrue(outcome.message().contains("manual-acceptance"));
    }

    @Test
    void srtSupportedKindsStartInsideTaskSandboxHomeAndDoNotLeakManagedEnv() throws Exception {
        List<Case> cases = List.of(
            new Case(ScriptRunnerKind.SHELL, "shell", "sh", "pwd\n"),
            new Case(ScriptRunnerKind.PYTHON, "python", "python3", "import os; print(os.getcwd())\n"),
            new Case(ScriptRunnerKind.POWERSHELL, "powershell", "pwsh", "Get-Location\n"),
            new Case(ScriptRunnerKind.RHAI, "rhai", "rhai-run", "print(\"ok\");\n"),
            new Case(ScriptRunnerKind.PHP, "php", "php", "<?php echo getcwd(); ?>\n"),
            new Case(ScriptRunnerKind.GROOVY, "groovy", "groovy", "println System.getProperty('user.dir')\n")
        );
        for (Case item : cases) {
            Path report = tempDir.resolve("srt-" + item.language + ".txt");
            Path fakeSrt = tempDir.resolve("srt-" + item.language);
            Files.writeString(
                fakeSrt,
                "#!/usr/bin/env sh\n" +
                    "printf 'cwd=%s\\n' \"$(pwd)\" > " + shellQuote(report.toString()) + "\n" +
                    "printf 'home=%s\\n' \"$HOME\" >> " + shellQuote(report.toString()) + "\n" +
                    "printf 'tmp=%s\\n' \"$TMPDIR\" >> " + shellQuote(report.toString()) + "\n" +
                    "printf 'claude_tmp=%s\\n' \"$CLAUDE_CODE_TMPDIR\" >> " + shellQuote(report.toString()) + "\n" +
                    "printf 'args=%s\\n' \"$*\" >> " + shellQuote(report.toString()) + "\n" +
                    "exit 0\n"
            );
            fakeSrt.toFile().setExecutable(true);
            SrtScriptRunner runner = new SrtScriptRunner(item.kind, fakeSrt.toString(), item.interpreter);

            TaskOutcome outcome = runner.run(task(item.language, item.content, List.of("HOME", "TMPDIR", "CLAUDE_CODE_TMPDIR")));

            assertTrue(outcome.success(), item.language + " outcome=" + outcome.message());
            Map<String, String> values = reportValues(report);
            assertTrue(values.get("cwd").equals(values.get("home")), item.language + " should start in sandbox HOME: " + values);
            assertTrue(values.get("home").contains("tikee-srt-" + item.kind.value() + "-runtime"), values.toString());
            Path runtimeRoot = Path.of(values.get("home")).getParent();
            assertTrue(Path.of(values.get("tmp")).equals(runtimeRoot.resolve("tmp")), values.toString());
            assertTrue(values.get("claude_tmp").equals(values.get("tmp")), values.toString());
            if (item.kind == ScriptRunnerKind.RHAI) {
                assertTrue(values.get("args").contains("/home/script-"), values.get("args"));
            }
        }
    }

    private record Case(ScriptRunnerKind kind, String language, String interpreter, String content) {}

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
        return task("shell", content);
    }

    private static ScriptRunnerTask task(String language, String content) throws Exception {
        return task(language, content, List.of());
    }

    private static ScriptRunnerTask task(String language, String content, List<String> allowedEnvVars) throws Exception {
        return new ScriptRunnerTask(
            "script-1",
            "sv-1",
            1,
            language,
            content,
            sha256(content),
            new ScriptRunnerPolicy(
                1000,
                1048576,
                1048576,
                false,
                List.of(),
                allowedEnvVars,
                List.of(),
                List.of(),
                List.of()
            ),
            ScriptSandboxBackend.AUTO
        );
    }

    private static Map<String, String> reportValues(Path report) throws Exception {
        Map<String, String> values = new java.util.LinkedHashMap<>();
        for (String line : Files.readString(report).split("\\R")) {
            int index = line.indexOf('=');
            if (index > 0) {
                values.put(line.substring(0, index), line.substring(index + 1));
            }
        }
        return values;
    }

    private static String shellQuote(String value) {
        return "'" + value.replace("'", "'\\''") + "'";
    }
}
