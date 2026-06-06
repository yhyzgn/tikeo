package net.tikeo.wasm;

import net.tikeo.processor.TaskOutcome;
import net.tikeo.script.ScriptRunnerLogSink;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;

/** CLI-backed Wasmtime sandbox runner for WASM processor bindings. */
public final class CliWasmtimeRunner implements WasmRunner {

    private final String runtimeCommand;
    private final List<String> runtimeArgs;

    public CliWasmtimeRunner() {
        this("wasmtime", List.of());
    }

    public CliWasmtimeRunner(String runtimeCommand, List<String> runtimeArgs) {
        this.runtimeCommand =
            runtimeCommand == null || runtimeCommand.isBlank()
                ? "wasmtime"
                : runtimeCommand;
        this.runtimeArgs = List.copyOf(
            runtimeArgs == null ? List.of() : runtimeArgs
        );
    }

    @Override
    public TaskOutcome run(WasmRunnerTask task) throws Exception {
        return run(task, ScriptRunnerLogSink.NOOP);
    }

    @Override
    public TaskOutcome run(WasmRunnerTask task, ScriptRunnerLogSink logSink)
        throws Exception {
        validateTask(task);
        Path module = Files.createTempFile("tikeo-wasm-", ".wasm");
        try {
            Files.write(module, task.module());
            ProcessBuilder builder = new ProcessBuilder(command(task, module));
            builder.redirectErrorStream(true);
            Process process = builder.start();
            CompletableFuture<byte[]> output = CompletableFuture.supplyAsync(
                () -> readAll(process.getInputStream())
            );
            if (
                !process.waitFor(
                    task.policy().timeoutMillis(),
                    TimeUnit.MILLISECONDS
                )
            ) {
                process.destroyForcibly();
                throw new WasmRunnerException(
                    "wasm timed out after " +
                        task.policy().timeoutMillis() +
                        "ms"
                );
            }
            byte[] bytes = output.get(1, TimeUnit.SECONDS);
            emitOutput(logSink, bytes);
            String message = new String(bytes, StandardCharsets.UTF_8).trim();
            if (process.exitValue() == 0) {
                return new TaskOutcome(
                    true,
                    message.isBlank() ? "wasm ok" : message
                );
            }
            return TaskOutcome.failed(
                message.isBlank()
                    ? "wasm exited with code " + process.exitValue()
                    : message
            );
        } finally {
            Files.deleteIfExists(module);
        }
    }

    List<String> command(WasmRunnerTask task, Path module) {
        List<String> command = new ArrayList<>();
        command.add(runtimeCommand);
        command.add("run");
        command.addAll(runtimeArgs);
        String entrypoint =
            task.entrypoint() == null ? "" : task.entrypoint().trim();
        if (!entrypoint.isBlank() && !"_start".equals(entrypoint)) {
            command.add("--invoke");
            command.add(entrypoint);
        }
        for (String name : task.policy().allowedEnvVars()) {
            String value = System.getenv(name);
            if (value != null) {
                command.add("--env");
                command.add(name + "=" + value);
            }
        }
        command.add(module.toString());
        return command;
    }

    private static void validateTask(WasmRunnerTask task) {
        if (task == null) {
            throw new WasmRunnerException("wasm task is required");
        }
        if (task.module() == null || task.module().length == 0) {
            throw new WasmRunnerException("wasm runner requires module bytes");
        }
        if (
            task.versionId() == null ||
            task.versionId().isBlank() ||
            task.versionNumber() <= 0
        ) {
            throw new WasmRunnerException(
                "wasm runner requires a released immutable module snapshot"
            );
        }
        if (task.moduleSha256() == null || task.moduleSha256().isBlank()) {
            throw new WasmRunnerException(
                "wasm runner requires module sha256 digest"
            );
        }
        if (!task.moduleSha256().equalsIgnoreCase(sha256(task.module()))) {
            throw new WasmRunnerException("wasm module sha256 digest mismatch");
        }
        if (task.policy().allowNetwork()) {
            throw new WasmRunnerException(
                "wasm runner does not grant network access by default"
            );
        }
    }

    private static byte[] readAll(InputStream input) {
        try (input) {
            return input.readAllBytes();
        } catch (IOException error) {
            throw new WasmRunnerException("failed to read wasm output", error);
        }
    }

    private static void emitOutput(ScriptRunnerLogSink logSink, byte[] bytes) {
        if (bytes.length == 0) {
            return;
        }
        String text = new String(bytes, StandardCharsets.UTF_8);
        for (String line : text.split("\\R")) {
            if (!line.isBlank()) {
                logSink.log("info", line);
            }
        }
    }

    private static String sha256(byte[] bytes) {
        try {
            return HexFormat.of().formatHex(
                MessageDigest.getInstance("SHA-256").digest(bytes)
            );
        } catch (NoSuchAlgorithmException error) {
            throw new WasmRunnerException("SHA-256 is not available", error);
        }
    }
}
