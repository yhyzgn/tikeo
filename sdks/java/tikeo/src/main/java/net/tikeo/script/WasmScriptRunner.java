package net.tikeo.script;

import net.tikeo.processor.TaskOutcome;
import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

/** Wasmtime-backed default sandbox runner for dynamic scripts. */
public final class WasmScriptRunner implements ScriptRunner {
    private static final String SHELL_RUNTIME_RESOURCE = "/tikeo/wasm-runtimes/shell/runner.wasm";

    private final ScriptRunnerKind kind;
    private final String runtimeCommand;
    private final List<String> runtimeArgs;

    public WasmScriptRunner(ScriptRunnerKind kind, String runtimeCommand, List<String> runtimeArgs) {
        this.kind = kind;
        this.runtimeCommand = runtimeCommand == null || runtimeCommand.isBlank() ? "wasmtime" : runtimeCommand;
        this.runtimeArgs = List.copyOf(runtimeArgs == null ? List.of() : runtimeArgs);
    }

    @Override
    public ScriptRunnerKind kind() {
        return kind;
    }

    @Override
    public ScriptSandboxBackend advertisedBackend() {
        return ScriptSandboxBackend.WASMTIME;
    }

    @Override
    public TaskOutcome run(ScriptRunnerTask task) {
        return run(task, ScriptRunnerLogSink.NOOP);
    }

    @Override
    public TaskOutcome run(ScriptRunnerTask task, ScriptRunnerLogSink logSink) {
        Path runtimeModule = null;
        try {
            runtimeModule = extractRuntimeModule();
            ProcessBuilder builder = new ProcessBuilder(command(task, runtimeModule));
            return ScriptRunnerSupport.runProcess(builder, kind, task, logSink);
        } catch (IOException error) {
            throw new ScriptRunnerException("failed to prepare WASM script runtime: " + error.getMessage(), error);
        } finally {
            if (runtimeModule != null) {
                try {
                    Files.deleteIfExists(runtimeModule);
                } catch (IOException ignored) {
                    // Temporary runtime cleanup failure does not change task outcome.
                }
            }
        }
    }

    List<String> command(ScriptRunnerTask task, Path runtimeModule) {
        ScriptRunnerSupport.validateTask(kind, task);
        validateSupportedCapabilities(task);
        if (runtimeModule == null) {
            throw new ScriptRunnerException("WASM script runner requires a runtime module");
        }
        ScriptSandboxBackend resolvedBackend = task.sandboxBackend().resolve(kind);
        if (resolvedBackend != ScriptSandboxBackend.WASMTIME) {
            throw new ScriptRunnerException(
                    "WASM script runner supports wasmtime backend only, requested: " + resolvedBackend.value());
        }
        List<String> command = new ArrayList<>();
        command.add(runtimeCommand);
        command.add("run");
        command.addAll(runtimeArgs);
        command.add("--env");
        command.add("TIKEO_SCRIPT_ID=" + task.scriptId());
        command.add("--env");
        command.add("TIKEO_SCRIPT_VERSION_ID=" + task.versionId());
        command.add("--env");
        command.add("TIKEO_SCRIPT_VERSION_NUMBER=" + task.versionNumber());
        for (String name : task.policy().allowedEnvVars()) {
            String value = System.getenv(name);
            if (value != null) {
                command.add("--env");
                command.add(name + "=" + value);
            }
        }
        command.add(runtimeModule.toString());
        return command;
    }

    private void validateSupportedCapabilities(ScriptRunnerTask task) {
        if (kind != ScriptRunnerKind.SHELL) {
            throw new ScriptRunnerException("WASM script runner currently supports shell scripts only");
        }
        if (task.policy().allowNetwork() || !task.policy().allowedNetworkHosts().isEmpty()) {
            throw new ScriptRunnerException("WASM script runner does not grant network access by default");
        }
        if (!task.policy().readOnlyPaths().isEmpty() || !task.policy().writablePaths().isEmpty()) {
            throw new ScriptRunnerException("WASM script runner does not grant filesystem access by default");
        }
        if (!task.policy().secretRefs().isEmpty()) {
            throw new ScriptRunnerException("WASM script runner cannot resolve script secret refs without a worker-local secret provider");
        }
    }

    private Path extractRuntimeModule() throws IOException {
        if (kind != ScriptRunnerKind.SHELL) {
            throw new ScriptRunnerException("WASM script runner currently supports shell scripts only");
        }
        try (InputStream input = WasmScriptRunner.class.getResourceAsStream(SHELL_RUNTIME_RESOURCE)) {
            if (input == null) {
                throw new ScriptRunnerException("bundled WASM shell runtime is missing");
            }
            Path module = Files.createTempFile("tikeo-wasm-script-shell-", ".wasm");
            Files.copy(input, module, java.nio.file.StandardCopyOption.REPLACE_EXISTING);
            return module;
        }
    }
}
