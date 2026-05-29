package com.yhyzgn.tikee.script;

import com.yhyzgn.tikee.processor.TaskOutcome;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

/** Development-only local subprocess runner for dynamic scripts. */
public final class LocalSubprocessScriptRunner implements ScriptRunner {
    private final ScriptRunnerKind kind;
    private final String command;
    private final List<String> args;

    public LocalSubprocessScriptRunner(ScriptRunnerKind kind) {
        this(kind, defaultScriptCommand(kind));
    }

    public LocalSubprocessScriptRunner(ScriptRunnerKind kind, List<String> command) {
        if (command == null || command.isEmpty() || command.getFirst() == null || command.getFirst().isBlank()) {
            throw new ScriptRunnerException("local script runner requires a command");
        }
        this.kind = kind;
        this.command = command.getFirst();
        this.args = List.copyOf(command.subList(1, command.size()));
    }

    @Override
    public ScriptRunnerKind kind() {
        return kind;
    }

    @Override
    public TaskOutcome run(ScriptRunnerTask task) {
        return run(task, ScriptRunnerLogSink.NOOP);
    }

    @Override
    public TaskOutcome run(ScriptRunnerTask task, ScriptRunnerLogSink logSink) {
        if (kind == ScriptRunnerKind.RHAI) {
            return runRhaiFile(task, logSink);
        }
        ProcessBuilder builder = new ProcessBuilder(command(task));
        configureEnvironment(builder, task);
        return ScriptRunnerSupport.runProcess(builder, kind, task, logSink);
    }

    private TaskOutcome runRhaiFile(ScriptRunnerTask task, ScriptRunnerLogSink logSink) {
        Path script = null;
        try {
            script = Files.createTempFile("tikee-rhai-script-", ".rhai");
            Files.writeString(script, task.content());
            java.util.ArrayList<String> command = new java.util.ArrayList<>(command(task));
            command.add(script.toString());
            ProcessBuilder builder = new ProcessBuilder(command);
            configureEnvironment(builder, task);
            return ScriptRunnerSupport.runProcessWithoutStdin(builder, kind, task, logSink);
        } catch (IOException error) {
            throw new ScriptRunnerException("failed to prepare rhai script file: " + error.getMessage(), error);
        } finally {
            if (script != null) {
                try {
                    Files.deleteIfExists(script);
                } catch (IOException ignored) {
                    // Best-effort cleanup only.
                }
            }
        }
    }

    private void configureEnvironment(ProcessBuilder builder, ScriptRunnerTask task) {
        builder.environment().clear();
        builder.environment().put("TIKEE_SCRIPT_ID", task.scriptId());
        builder.environment().put("TIKEE_SCRIPT_VERSION_ID", task.versionId());
        builder.environment().put("TIKEE_SCRIPT_VERSION_NUMBER", Long.toString(task.versionNumber()));
        for (String name : task.policy().allowedEnvVars()) {
            String value = System.getenv(name);
            if (value != null) {
                builder.environment().put(name, value);
            }
        }
    }

    List<String> command(ScriptRunnerTask task) {
        ScriptRunnerSupport.validateTask(kind, task);
        validateDevelopmentOnlyPolicy(task);
        java.util.ArrayList<String> resolved = new java.util.ArrayList<>();
        resolved.add(command);
        resolved.addAll(args);
        return resolved;
    }

    private void validateDevelopmentOnlyPolicy(ScriptRunnerTask task) {
        ScriptSandboxBackend resolvedBackend = task.sandboxBackend().resolve(kind);
        if (!supportsBackend(resolvedBackend)) {
            throw new ScriptRunnerException(
                    "local script runner does not support backend for "
                            + kind.value()
                            + ": "
                            + resolvedBackend.value());
        }
        if (task.policy().allowNetwork() || !task.policy().allowedNetworkHosts().isEmpty()) {
            throw new ScriptRunnerException("local script runner does not grant network access");
        }
        if (!task.policy().readOnlyPaths().isEmpty() || !task.policy().writablePaths().isEmpty()) {
            throw new ScriptRunnerException("local script runner does not grant filesystem access");
        }
        if (!task.policy().secretRefs().isEmpty()) {
            throw new ScriptRunnerException("local script runner cannot resolve script secret refs");
        }
    }

    private boolean supportsBackend(ScriptSandboxBackend backend) {
        return switch (kind) {
            case JS, TS -> backend == ScriptSandboxBackend.DENO
                    || backend == ScriptSandboxBackend.V8
                    || backend == ScriptSandboxBackend.CUSTOM;
            case SHELL, PYTHON, POWERSHELL, PHP, GROOVY, RHAI -> backend == ScriptSandboxBackend.SRT
                    || backend == ScriptSandboxBackend.CUSTOM;
        };
    }

    private static List<String> defaultScriptCommand(ScriptRunnerKind kind) {
        return switch (kind) {
            case SHELL -> List.of("sh", "-s");
            case PYTHON -> List.of("python3", "-");
            case JS, TS -> List.of("deno", "run", "--no-prompt", "-");
            case POWERSHELL -> List.of("pwsh", "-NoProfile", "-NonInteractive", "-Command", "-");
            case PHP -> List.of("php");
            case GROOVY -> List.of("groovy");
            case RHAI -> List.of("rhai");
        };
    }
}
