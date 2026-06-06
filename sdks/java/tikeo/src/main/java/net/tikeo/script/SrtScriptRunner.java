package net.tikeo.script;

import com.fasterxml.jackson.databind.ObjectMapper;
import net.tikeo.processor.TaskOutcome;
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
    private final List<String> extraPathEntries;

    public SrtScriptRunner(ScriptRunnerKind kind, String runtimeCommand) {
        this(kind, runtimeCommand, defaultInterpreterCommand(kind));
    }

    public SrtScriptRunner(
        ScriptRunnerKind kind,
        String runtimeCommand,
        String interpreterCommand
    ) {
        this(kind, runtimeCommand, interpreterCommand, List.of());
    }

    public SrtScriptRunner(
        ScriptRunnerKind kind,
        String runtimeCommand,
        String interpreterCommand,
        List<String> extraPathEntries
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
        this.extraPathEntries = List.copyOf(extraPathEntries == null ? List.of() : extraPathEntries);
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
        try (TaskRuntimeDirs runtimeDirs = TaskRuntimeDirs.create("tikeo-srt-" + kind.value() + "-runtime")) {
            ScriptRunnerSupport.validateTask(kind, task);
            validatePolicy(task);
            Path scriptFile = null;
            if (kind == ScriptRunnerKind.RHAI) {
                scriptFile = runtimeDirs.scriptFile("rhai");
                Files.writeString(scriptFile, task.content());
            }
            settings = writeSettings(task.policy(), runtimeDirs, scriptFile);
            ProcessBuilder builder = new ProcessBuilder(
                command(settings, shellCommand(task.content(), scriptFile))
            );
            builder.directory(runtimeDirs.workingDir().toFile());
            configureEnvironment(builder, task, runtimeDirs);
            return ScriptRunnerSupport.runProcessWithoutStdin(builder, kind, task, logSink);
        } catch (IOException error) {
            throw new ScriptRunnerException("failed to prepare SRT runtime: " + error.getMessage(), error);
        } finally {
            deleteIfPresent(settings);
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
                "cd \"$HOME\" && " + interpreterCommand + " -NoLogo -NoProfile -NonInteractive -InputFormat Text -OutputFormat Text -Command -",
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
            delimiter = delimiter + "_TIKEO";
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

    private void configureEnvironment(ProcessBuilder builder, ScriptRunnerTask task, TaskRuntimeDirs runtimeDirs) {
        builder.environment().clear();
        runtimeDirs.applySrtEnvironment(builder, extraPathEntries);
        if (kind == ScriptRunnerKind.POWERSHELL) {
            runtimeDirs.applyPowerShellEnvironment(builder);
        }
        builder.environment().put("TIKEO_SCRIPT_ID", task.scriptId());
        builder.environment().put("TIKEO_SCRIPT_VERSION_ID", task.versionId());
        builder
            .environment()
            .put(
                "TIKEO_SCRIPT_VERSION_NUMBER",
                Long.toString(task.versionNumber())
            );
        runtimeDirs.appendAllowedUnmanagedEnv(builder, task.policy().allowedEnvVars());
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

    private static Path writeSettings(ScriptRunnerPolicy policy, TaskRuntimeDirs runtimeDirs, Path scriptFile) throws IOException {
        Map<String, Object> network = new LinkedHashMap<>();
        network.put("allowUnixSocket", false);
        network.put("allowedDomains", policy.allowedNetworkHosts());
        network.put("deniedDomains", List.of());

        Map<String, Object> filesystem = new LinkedHashMap<>();
        java.util.ArrayList<String> allowRead = new java.util.ArrayList<>(
            policy.readOnlyPaths()
        );
        if (scriptFile != null) {
            allowRead.add(scriptFile.toString());
        }
        filesystem.put("allowRead", allowRead);
        java.util.ArrayList<String> allowWrite = new java.util.ArrayList<>(policy.writablePaths());
        allowWrite.addAll(runtimeDirs.writablePaths());
        filesystem.put("allowWrite", allowWrite);
        filesystem.put("denyRead", sensitiveReadDenies());
        filesystem.put("denyWrite", List.of());

        Map<String, Object> settings = new LinkedHashMap<>();
        settings.put("network", network);
        settings.put("filesystem", filesystem);

        Path file = Files.createTempFile("tikeo-srt-settings-", ".json");
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

    private static List<String> sensitiveReadDenies() {
        String home = System.getProperty("user.home", "");
        if (home.isBlank()) {
            return List.of();
        }
        return List.of(
            Path.of(home, ".ssh").toString(),
            Path.of(home, ".gnupg").toString(),
            Path.of(home, ".aws").toString(),
            Path.of(home, ".kube").toString(),
            Path.of(home, ".docker").toString(),
            Path.of(home, ".config", "tikeo").toString()
        );
    }
}
