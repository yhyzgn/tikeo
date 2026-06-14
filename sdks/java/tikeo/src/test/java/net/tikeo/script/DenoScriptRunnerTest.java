package net.tikeo.script;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.tikeo.processor.TaskOutcome;
import org.junit.jupiter.api.Assertions;
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
            task("javascript", "console.log('deno-v8-ok');", ScriptSandboxBackend.V8, List.of()),
            (level, message) -> logs.add(level + ":" + message)
        );

        Assertions.assertTrue(outcome.success(), outcome.message());
        Assertions.assertTrue(
            logs.stream().anyMatch(log -> log.equals("info:[script] deno-v8-ok"))
        );
    }

    @Test
    void denoStartsJavaScriptAndTypeScriptInsideTaskSandboxHome() throws Exception {
        for (Case item : List.of(
            new Case(ScriptRunnerKind.JS, "javascript"),
            new Case(ScriptRunnerKind.TS, "typescript")
        )) {
            Path report = tempDir.resolve("deno-" + item.language + ".txt");
            Path fakeDeno = tempDir.resolve("deno-" + item.language);
            Files.writeString(
                fakeDeno,
                "#!/usr/bin/env sh\n" +
                    "cat >/dev/null\n" +
                    "printf 'cwd=%s\\n' \"$(pwd)\" > " + shellQuote(report.toString()) + "\n" +
                    "printf 'home=%s\\n' \"$HOME\" >> " + shellQuote(report.toString()) + "\n" +
                    "printf 'tmp=%s\\n' \"$TMPDIR\" >> " + shellQuote(report.toString()) + "\n" +
                    "printf 'deno_dir=%s\\n' \"$DENO_DIR\" >> " + shellQuote(report.toString()) + "\n" +
                    "printf 'args=%s\\n' \"$*\" >> " + shellQuote(report.toString()) + "\n" +
                    "exit 0\n"
            );
            fakeDeno.toFile().setExecutable(true);
            DenoScriptRunner runner = new DenoScriptRunner(item.kind, fakeDeno.toString());

            TaskOutcome outcome = runner.run(task(item.language, "console.log('ok');\n", ScriptSandboxBackend.AUTO, List.of("HOME", "TMPDIR", "DENO_DIR")));

            Assertions.assertTrue(outcome.success(), item.language + " outcome=" + outcome.message());
            Map<String, String> values = reportValues(report);
            Assertions.assertTrue(values.get("cwd").equals(values.get("home")), item.language + " should start in sandbox HOME: " + values);
            Assertions.assertTrue(values.get("home").contains("tikeo-deno-" + item.kind.value() + "-runtime"), values.toString());
            Path runtimeRoot = Path.of(values.get("home")).getParent();
            Assertions.assertTrue(Path.of(values.get("tmp")).equals(runtimeRoot.resolve("tmp")), values.toString());
            Assertions.assertTrue(Path.of(values.get("deno_dir")).equals(runtimeRoot.resolve("cache").resolve("deno")), values.toString());
            Assertions.assertTrue(values.get("args").contains("run --no-prompt"), values.get("args"));
        }
    }

    private record Case(ScriptRunnerKind kind, String language) {}

    private static ScriptRunnerTask task(
        String language,
        String content,
        ScriptSandboxBackend backend,
        List<String> allowedEnvVars
    ) throws Exception {
        return new ScriptRunnerTask(
            "script-1",
            "sv-1",
            1,
            language,
            content,
            ScriptRunnerSupport.sha256Hex(content),
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
            backend
        );
    }

    private static Map<String, String> reportValues(Path report) throws Exception {
        Map<String, String> values = new LinkedHashMap<>();
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
