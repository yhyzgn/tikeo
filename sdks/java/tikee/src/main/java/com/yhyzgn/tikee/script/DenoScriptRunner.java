package com.yhyzgn.tikee.script;

import com.yhyzgn.tikee.processor.TaskOutcome;
import java.util.ArrayList;
import java.util.List;

/** Deno-backed sandbox runner for JavaScript and TypeScript scripts. */
public final class DenoScriptRunner implements ScriptRunner {
    private final ScriptRunnerKind kind;
    private final String runtimeCommand;

    public DenoScriptRunner(ScriptRunnerKind kind, String runtimeCommand) {
        if (kind != ScriptRunnerKind.JS && kind != ScriptRunnerKind.TS) {
            throw new ScriptRunnerException("Deno script runner supports JavaScript and TypeScript only");
        }
        if (runtimeCommand == null || runtimeCommand.isBlank()) {
            throw new ScriptRunnerException("Deno script runner requires a runtime command");
        }
        this.kind = kind;
        this.runtimeCommand = runtimeCommand;
    }

    @Override
    public ScriptRunnerKind kind() {
        return kind;
    }

    @Override
    public ScriptSandboxBackend advertisedBackend() {
        return kind == ScriptRunnerKind.JS || kind == ScriptRunnerKind.TS ? ScriptSandboxBackend.DENO : ScriptSandboxBackend.AUTO;
    }

    @Override
    public TaskOutcome run(ScriptRunnerTask task) {
        return run(task, ScriptRunnerLogSink.NOOP);
    }

    @Override
    public TaskOutcome run(ScriptRunnerTask task, ScriptRunnerLogSink logSink) {
        ScriptRunnerSupport.validateTask(kind, task);
        validatePolicy(task);
        try (TaskRuntimeDirs runtimeDirs = TaskRuntimeDirs.create("tikee-deno-" + kind.value() + "-runtime")) {
            ProcessBuilder builder = new ProcessBuilder(command(task, runtimeDirs));
            builder.directory(runtimeDirs.workingDir().toFile());
            configureEnvironment(builder, task, runtimeDirs);
            return ScriptRunnerSupport.runProcess(builder, kind, task, logSink);
        } catch (java.io.IOException error) {
            throw new ScriptRunnerException("failed to prepare Deno runtime: " + error.getMessage(), error);
        }
    }

    private List<String> command(ScriptRunnerTask task, TaskRuntimeDirs runtimeDirs) {

        List<String> command = new ArrayList<>();
        command.add(runtimeCommand);
        command.add("run");
        command.add("--no-prompt");
        if (task.policy().allowNetwork()) {
            command.add("--allow-net");
        } else if (!task.policy().allowedNetworkHosts().isEmpty()) {
            command.add("--allow-net=" + String.join(",", task.policy().allowedNetworkHosts()));
        }
        if (!task.policy().allowedEnvVars().isEmpty()) {
            command.add("--allow-env=" + String.join(",", task.policy().allowedEnvVars()));
        }
        if (!task.policy().readOnlyPaths().isEmpty()) {
            command.add("--allow-read=" + String.join(",", task.policy().readOnlyPaths()));
        }
        List<String> writablePaths = new ArrayList<>(task.policy().writablePaths());
        writablePaths.addAll(runtimeDirs.writablePaths());
        if (!writablePaths.isEmpty()) {
            command.add("--allow-write=" + String.join(",", writablePaths));
        }
        command.add("-");
        return command;
    }

    private void configureEnvironment(ProcessBuilder builder, ScriptRunnerTask task, TaskRuntimeDirs runtimeDirs) {
        builder.environment().clear();
        runtimeDirs.applyDenoEnvironment(builder);
        builder.environment().put("TIKEE_SCRIPT_ID", task.scriptId());
        builder.environment().put("TIKEE_SCRIPT_VERSION_ID", task.versionId());
        builder
            .environment()
            .put(
                "TIKEE_SCRIPT_VERSION_NUMBER",
                Long.toString(task.versionNumber())
            );
        runtimeDirs.appendAllowedUnmanagedEnv(builder, task.policy().allowedEnvVars());
    }

    private void validatePolicy(ScriptRunnerTask task) {
        ScriptSandboxBackend backend = task.sandboxBackend().resolve(kind);
        if (
            backend != ScriptSandboxBackend.DENO &&
            backend != ScriptSandboxBackend.V8 &&
            backend != ScriptSandboxBackend.CUSTOM
        ) {
            throw new ScriptRunnerException(
                "Deno/V8 script runner does not support backend: " + backend.value()
            );
        }
        if (!task.policy().secretRefs().isEmpty()) {
            throw new ScriptRunnerException(
                "Deno script runner cannot resolve script secret refs without a worker-local secret provider"
            );
        }
    }
}
