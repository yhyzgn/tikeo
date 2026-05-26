package com.yhyzgn.tikee.script;

import com.yhyzgn.tikee.processor.TaskOutcome;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.time.Duration;
import java.util.HexFormat;
import java.util.concurrent.TimeUnit;

final class ScriptRunnerSupport {
    private ScriptRunnerSupport() {}

    static void validateTask(ScriptRunnerKind kind, ScriptRunnerTask task) {
        task.policy().validateResourceLimits();
        ScriptRunnerKind taskKind = ScriptRunnerKind.fromLanguage(task.language())
                .orElseThrow(() -> new ScriptRunnerException("unsupported script language: " + task.language()));
        if (taskKind != kind) {
            throw new ScriptRunnerException(kind.value() + " runner cannot execute " + task.language() + " scripts");
        }
        if (task.versionId() == null || task.versionId().isBlank() || task.versionNumber() <= 0) {
            throw new ScriptRunnerException("script runner requires a released immutable script version snapshot");
        }
        if (task.contentSha256() == null || task.contentSha256().isBlank()) {
            throw new ScriptRunnerException("script runner requires a content sha256 digest");
        }
        String actual = sha256Hex(task.content());
        if (!actual.equalsIgnoreCase(task.contentSha256().trim())) {
            throw new ScriptRunnerException("script content sha256 digest mismatch");
        }
    }

    static TaskOutcome runProcess(
            ProcessBuilder processBuilder,
            ScriptRunnerKind kind,
            ScriptRunnerTask task,
            ScriptRunnerLogSink logSink) {
        ScriptRunnerLogSink sink = logSink == null ? ScriptRunnerLogSink.NOOP : logSink;
        try {
            Process process = processBuilder.start();
            Thread stdoutReader = streamReader(process.getInputStream());
            Thread stderrReader = streamReader(process.getErrorStream());
            process.getOutputStream().write(task.content().getBytes(StandardCharsets.UTF_8));
            process.getOutputStream().close();

            boolean completed = process.waitFor(task.policy().timeoutMillis(), TimeUnit.MILLISECONDS);
            if (!completed) {
                process.destroyForcibly();
                throw new ScriptRunnerException("script timed out after " + task.policy().timeoutMillis() + "ms");
            }
            stdoutReader.join(Duration.ofSeconds(1).toMillis());
            stderrReader.join(Duration.ofSeconds(1).toMillis());
            byte[] stdout = ((CapturingThread) stdoutReader).bytes();
            byte[] stderr = ((CapturingThread) stderrReader).bytes();
            long outputBytes = (long) stdout.length + stderr.length;
            if (outputBytes > task.policy().maxOutputBytes()) {
                throw new ScriptRunnerException("script output exceeded " + task.policy().maxOutputBytes()
                        + " bytes: " + outputBytes);
            }
            emitCapturedOutput(sink, "info", stdout);
            emitCapturedOutput(sink, process.exitValue() == 0 ? "warn" : "error", stderr);
            if (process.exitValue() == 0) {
                return TaskOutcome.succeeded();
            }
            String message = new String(stderr.length == 0 ? stdout : stderr, StandardCharsets.UTF_8).trim();
            return TaskOutcome.failed(message.isEmpty()
                    ? kind.value() + " runner exited with status " + process.exitValue()
                    : message);
        } catch (IOException error) {
            throw new ScriptRunnerException("failed to spawn " + kind.value() + " runner: " + error.getMessage(), error);
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            throw new ScriptRunnerException("script runner interrupted", error);
        }
    }

    private static void emitCapturedOutput(ScriptRunnerLogSink logSink, String level, byte[] bytes) {
        if (bytes.length == 0) {
            return;
        }
        String text = new String(bytes, StandardCharsets.UTF_8).replace("\r\n", "\n");
        for (String line : text.split("\n", -1)) {
            if (!line.isBlank()) {
                logSink.log(level, "[script] " + line);
            }
        }
    }

    static String sha256Hex(String content) {
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            return HexFormat.of().formatHex(digest.digest(content.getBytes(StandardCharsets.UTF_8)));
        } catch (NoSuchAlgorithmException error) {
            throw new IllegalStateException(error);
        }
    }

    private static CapturingThread streamReader(java.io.InputStream input) {
        CapturingThread thread = new CapturingThread(input);
        thread.setDaemon(true);
        thread.start();
        return thread;
    }

    private static final class CapturingThread extends Thread {
        private final java.io.InputStream input;
        private final ByteArrayOutputStream output = new ByteArrayOutputStream();

        private CapturingThread(java.io.InputStream input) {
            super("tikee-script-output-reader");
            this.input = input;
        }

        @Override
        public void run() {
            try (input) {
                input.transferTo(output);
            } catch (IOException ignored) {
                // The process may be killed on timeout; the caller reports the timeout.
            }
        }

        private byte[] bytes() {
            return output.toByteArray();
        }
    }
}
