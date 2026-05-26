package com.yhyzgn.tikee.script;

import com.yhyzgn.tikee.processor.TaskOutcome;
import java.nio.file.InvalidPathException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

/** Docker-compatible sandbox runner for dynamic scripts. */
public final class ContainerScriptRunner implements ScriptRunner {
    private final ScriptRunnerKind kind;
    private final String runtimeCommand;
    private final String image;
    private final List<String> runtimeArgs;

    public ContainerScriptRunner(ScriptRunnerKind kind, String image) {
        this(kind, "docker", image, List.of());
    }

    public ContainerScriptRunner(ScriptRunnerKind kind, String runtimeCommand, String image, List<String> runtimeArgs) {
        this.kind = kind;
        this.runtimeCommand = runtimeCommand == null || runtimeCommand.isBlank() ? "docker" : runtimeCommand;
        this.image = image;
        this.runtimeArgs = List.copyOf(runtimeArgs == null ? List.of() : runtimeArgs);
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
        return ScriptRunnerSupport.runProcess(builder, kind, task, logSink);
    }

    List<String> command(ScriptRunnerTask task) {
        ScriptRunnerSupport.validateTask(kind, task);
        validateSupportedCapabilities(task);
        if (image == null || image.isBlank()) {
            throw new ScriptRunnerException("container script runner requires an image");
        }
        List<String> command = new ArrayList<>();
        command.add(runtimeCommand);
        command.add("run");
        command.add("--rm");
        command.add("-i");
        command.add("--network=none");
        command.add("--read-only");
        command.add("--tmpfs");
        command.add("/tmp:rw,noexec,nosuid,size=16m");
        command.add("--memory=" + task.policy().maxMemoryBytes());
        command.add("--env");
        command.add("TIKEE_SCRIPT_ID=" + task.scriptId());
        command.add("--env");
        command.add("TIKEE_SCRIPT_VERSION_ID=" + task.versionId());
        command.add("--env");
        command.add("TIKEE_SCRIPT_VERSION_NUMBER=" + task.versionNumber());
        command.addAll(runtimeArgs);
        addFileMounts(command, task.policy().readOnlyPaths(), true);
        addFileMounts(command, task.policy().writablePaths(), false);
        for (String name : task.policy().allowedEnvVars()) {
            String value = System.getenv(name);
            if (value != null) {
                command.add("--env");
                command.add(name + "=" + value);
            }
        }
        command.add(image);
        command.addAll(defaultScriptCommand(kind));
        return command;
    }

    private void validateSupportedCapabilities(ScriptRunnerTask task) {
        if (task.policy().allowNetwork() || !task.policy().allowedNetworkHosts().isEmpty()) {
            throw new ScriptRunnerException(
                    "container script runner cannot safely enforce host-level network grants with Docker CLI alone");
        }
        if (!task.policy().secretRefs().isEmpty()) {
            throw new ScriptRunnerException(
                    "container script runner cannot resolve script secret refs without a worker-local secret provider");
        }
    }

    private static void addFileMounts(List<String> command, List<String> paths, boolean readOnly) {
        for (String path : paths) {
            Path clean = validateMountPath(path);
            command.add("--mount");
            command.add("type=bind,src=" + clean + ",dst=" + clean + (readOnly ? ",readonly" : ""));
        }
    }

    private static Path validateMountPath(String path) {
        try {
            Path candidate = Path.of(path == null ? "" : path);
            if (path == null || path.isBlank() || !path.equals(path.trim()) || !candidate.isAbsolute()
                    || candidate.normalize().compareTo(candidate) != 0) {
                throw new ScriptRunnerException("script file grant path must be clean and absolute: " + path);
            }
            return candidate;
        } catch (InvalidPathException error) {
            throw new ScriptRunnerException("script file grant path must be clean and absolute: " + path, error);
        }
    }

    private static List<String> defaultScriptCommand(ScriptRunnerKind kind) {
        return switch (kind) {
            case SHELL -> List.of("sh", "-s");
            case PYTHON -> List.of("python3", "-");
            case NODE -> List.of("node", "-");
            case POWERSHELL -> List.of("pwsh", "-NoProfile", "-NonInteractive", "-Command", "-");
        };
    }
}
