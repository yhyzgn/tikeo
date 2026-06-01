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

    public SrtScriptRunner(ScriptRunnerKind kind, String runtimeCommand) {
        if (runtimeCommand == null || runtimeCommand.isBlank()) {
            throw new ScriptRunnerException("SRT script runner requires a runtime command");
        }
        this.kind = kind;
        this.runtimeCommand = runtimeCommand;
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
        Path settings = null;
        try {
            ScriptRunnerSupport.validateTask(kind, task);
            validatePolicy(task);
            settings = writeSettings(task.policy());
            ProcessBuilder builder = new ProcessBuilder(
                command(settings, task.content())
            );
            configureEnvironment(builder, task);
            return ScriptRunnerSupport.runProcessWithoutStdin(builder, kind, task, logSink);
        } catch (IOException error) {
            throw new ScriptRunnerException("failed to prepare SRT settings: " + error.getMessage(), error);
        } finally {
            if (settings != null) {
                try {
                    Files.deleteIfExists(settings);
                } catch (IOException ignored) {
                    // Best-effort cleanup only.
                }
            }
        }
    }

    private List<String> command(Path settings, String content) {
        java.util.ArrayList<String> resolved = new java.util.ArrayList<>();
        resolved.add(runtimeCommand);
        resolved.add("--settings");
        resolved.add(settings.toString());
        resolved.add("-c");
        resolved.add(content);
        return resolved;
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

    private static Path writeSettings(ScriptRunnerPolicy policy) throws IOException {
        Map<String, Object> network = new LinkedHashMap<>();
        network.put("allowUnixSocket", false);
        network.put("allowedDomains", policy.allowedNetworkHosts());

        Map<String, Object> filesystem = new LinkedHashMap<>();
        filesystem.put("allowRead", policy.readOnlyPaths());
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

    private static String homeDirectory() {
        return System.getProperty("user.home", "");
    }
}
