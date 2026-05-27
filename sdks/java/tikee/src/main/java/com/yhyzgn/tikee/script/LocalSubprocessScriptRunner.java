package com.yhyzgn.tikee.script;

import com.yhyzgn.tikee.processor.TaskOutcome;
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
        ProcessBuilder builder = new ProcessBuilder(command(task));
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
        return ScriptRunnerSupport.runProcess(builder, kind, task, logSink);
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
        if (resolvedBackend != ScriptSandboxBackend.SRT && resolvedBackend != ScriptSandboxBackend.CUSTOM) {
            throw new ScriptRunnerException(
                    "local script runner supports development srt/custom backend only, requested: "
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

    private static List<String> defaultScriptCommand(ScriptRunnerKind kind) {
        return switch (kind) {
            case SHELL -> List.of("sh", "-s");
            case PYTHON -> List.of("python3", "-");
            case JS -> List.of("node", "-");
            case TS -> List.of("node", "-");
            case POWERSHELL -> List.of("pwsh", "-NoProfile", "-NonInteractive", "-Command", "-");
        };
    }
}
