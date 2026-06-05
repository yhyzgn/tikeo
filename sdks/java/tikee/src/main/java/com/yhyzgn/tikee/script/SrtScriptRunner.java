package com.yhyzgn.tikee.script;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.yhyzgn.tikee.processor.TaskOutcome;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** Anthropic Sandbox Runtime backed runner for native dynamic scripts. */
public final class SrtScriptRunner implements ScriptRunner {
    private static final ObjectMapper JSON = new ObjectMapper();

    private final ScriptRunnerKind kind;
    private final String runtimeCommand;
    private final String interpreterCommand;

    public SrtScriptRunner(ScriptRunnerKind kind, String runtimeCommand) {
        this(kind, runtimeCommand, defaultInterpreterCommand(kind));
    }

    public SrtScriptRunner(
        ScriptRunnerKind kind,
        String runtimeCommand,
        String interpreterCommand
    ) {
        if (runtimeCommand == null || runtimeCommand.isBlank()) {
            throw new ScriptRunnerException("SRT script runner requires a runtime command");
        }
        if (interpreterCommand == null || interpreterCommand.isBlank()) {
            throw new ScriptRunnerException("SRT script runner requires an interpreter command");
        }
        this.kind = kind;
        this.runtimeCommand = runtimeCommand;
        this.interpreterCommand = interpreterCommand;
    }

    @Override
    public ScriptRunnerKind kind() {
        return kind;
    }

    @Override
    public ScriptSandboxBackend advertisedBackend() {
        return ScriptSandboxBackend.SRT;
    }

    @Override
    public TaskOutcome run(ScriptRunnerTask task) {
        return run(task, ScriptRunnerLogSink.NOOP);
    }

    @Override
    public TaskOutcome run(ScriptRunnerTask task, ScriptRunnerLogSink logSink) {
        Path settings = null;
        Path scriptFile = null;
        try {
            ScriptRunnerSupport.validateTask(kind, task);
            validatePolicy(task);
            if (kind == ScriptRunnerKind.RHAI) {
                scriptFile = Files.createTempFile("tikee-rhai-script-", ".rhai");
                Files.writeString(scriptFile, task.content());
            }
            settings = writeSettings(task.policy(), scriptFile);
            ProcessBuilder builder = new ProcessBuilder(
                command(settings, shellCommand(task.content(), scriptFile))
            );
            configureEnvironment(builder, task);
            return ScriptRunnerSupport.runProcessWithoutStdin(builder, kind, task, logSink);
        } catch (IOException error) {
            throw new ScriptRunnerException("failed to prepare SRT settings: " + error.getMessage(), error);
        } finally {
            deleteIfPresent(settings);
            deleteIfPresent(scriptFile);
        }
    }

    private List<String> command(Path settings, String shellCommand) {
        java.util.ArrayList<String> resolved = new java.util.ArrayList<>();
        resolved.add(runtimeCommand);
        resolved.add("--settings");
        resolved.add(settings.toString());
        resolved.add("-c");
        resolved.add(shellCommand);
        return resolved;
    }

    private String shellCommand(String content, Path scriptFile) {
        return switch (kind) {
            case SHELL -> content;
            case PYTHON -> heredoc(interpreterCommand + " -", "PY", content);
            case POWERSHELL -> heredoc(
                interpreterCommand + " -NoProfile -NonInteractive -Command -",
                "PWSH",
                content
            );
            case PHP -> heredoc(interpreterCommand, "PHP", content);
            case GROOVY -> heredoc(interpreterCommand, "GROOVY", content);
            case RHAI -> interpreterCommand + " " + shellQuote(scriptFile.toString());
            case JS, TS -> throw new ScriptRunnerException(
                "SRT script runner does not execute " + kind.value() +
                    " scripts; use the Deno script runner"
            );
        };
    }

    private static String heredoc(String command, String marker, String content) {
        String delimiter = marker;
        while (content.contains(delimiter)) {
            delimiter = delimiter + "_TIKEE";
        }
        return command + " <<'" + delimiter + "'\n" + content + "\n" + delimiter;
    }

    private static String shellQuote(String value) {
        return "'" + value.replace("'", "'\''") + "'";
    }


    private static String defaultInterpreterCommand(ScriptRunnerKind kind) {
        return switch (kind) {
            case SHELL -> "sh";
            case PYTHON -> "python3";
            case POWERSHELL -> "pwsh";
            case PHP -> "php";
            case GROOVY -> "groovy";
            case RHAI -> "rhai-run";
            case JS, TS -> "deno";
        };
    }

    private void configureEnvironment(ProcessBuilder builder, ScriptRunnerTask task) {
        builder.environment().clear();
        String path = System.getenv("PATH");
        if (path != null) {
            builder.environment().put("PATH", path);
        }
        String home = System.getenv("HOME");
        if (home != null) {
            builder.environment().put("HOME", home);
        }
        builder.environment().put("TIKEE_SCRIPT_ID", task.scriptId());
        builder.environment().put("TIKEE_SCRIPT_VERSION_ID", task.versionId());
        builder
            .environment()
            .put(
                "TIKEE_SCRIPT_VERSION_NUMBER",
                Long.toString(task.versionNumber())
            );
        for (String name : task.policy().allowedEnvVars()) {
            String value = System.getenv(name);
            if (value != null) {
                builder.environment().put(name, value);
            }
        }
    }

    private void validatePolicy(ScriptRunnerTask task) {
        ScriptSandboxBackend resolvedBackend = task.sandboxBackend().resolve(kind);
        if (
            resolvedBackend != ScriptSandboxBackend.SRT &&
            resolvedBackend != ScriptSandboxBackend.CUSTOM
        ) {
            throw new ScriptRunnerException(
                "SRT script runner does not support backend: " +
                    resolvedBackend.value()
            );
        }
        if (!task.policy().secretRefs().isEmpty()) {
            throw new ScriptRunnerException(
                "SRT script runner cannot resolve script secret refs without a worker-local secret provider"
            );
        }
    }

    private static Path writeSettings(ScriptRunnerPolicy policy, Path scriptFile) throws IOException {
        Map<String, Object> network = new LinkedHashMap<>();
        network.put("allowUnixSocket", false);
        network.put("allowedDomains", policy.allowedNetworkHosts());

        Map<String, Object> filesystem = new LinkedHashMap<>();
        java.util.ArrayList<String> allowRead = new java.util.ArrayList<>(
            policy.readOnlyPaths()
        );
        if (scriptFile != null) {
            allowRead.add(scriptFile.toString());
        }
        filesystem.put("allowRead", allowRead);
        filesystem.put("allowWrite", policy.writablePaths());
        filesystem.put(
            "denyRead",
            policy.readOnlyPaths().isEmpty()
                ? List.of(homeDirectory())
                : List.of()
        );
        filesystem.put("denyWrite", List.of(homeDirectory()));

        Map<String, Object> settings = new LinkedHashMap<>();
        settings.put("network", network);
        settings.put("filesystem", filesystem);

        Path file = Files.createTempFile("tikee-srt-settings-", ".json");
        JSON.writeValue(file.toFile(), settings);
        return file;
    }

    private static void deleteIfPresent(Path path) {
        if (path == null) {
            return;
        }
        try {
            Files.deleteIfExists(path);
        } catch (IOException ignored) {
            // Best-effort cleanup only.
        }
    }

    private static String homeDirectory() {
        return System.getProperty("user.home", "");
    }
}
