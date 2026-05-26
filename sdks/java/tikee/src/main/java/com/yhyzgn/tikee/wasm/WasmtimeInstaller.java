package com.yhyzgn.tikee.wasm;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.Locale;
import java.util.concurrent.TimeUnit;

/** Installs a local Wasmtime runtime for Java workers when no runtime is already present. */
public final class WasmtimeInstaller {
    private WasmtimeInstaller() {}

    public record Options(
            String installVersion,
            Path installDir,
            String installerUrl,
            long installTimeoutMillis) {}

    public static Path install(Options options) {
        try {
            Files.createDirectories(options.installDir());
            ProcessBuilder builder = new ProcessBuilder(
                    "bash",
                    "-c",
                    "curl -fsSL " + shellQuote(options.installerUrl()) + " | bash -s -- --version "
                            + shellQuote(options.installVersion()));
            builder.environment().put("WASMTIME_HOME", options.installDir().toString());
            builder.environment().put("PROFILE", "/dev/null");
            builder.redirectErrorStream(true);
            builder.redirectOutput(ProcessBuilder.Redirect.DISCARD);
            Process process = builder.start();
            if (!process.waitFor(options.installTimeoutMillis(), TimeUnit.MILLISECONDS)) {
                process.destroyForcibly();
                throw new IllegalStateException(
                        "wasmtime install timed out after " + options.installTimeoutMillis() + "ms");
            }
            if (process.exitValue() != 0) {
                throw new IllegalStateException("wasmtime installer exited with code " + process.exitValue());
            }
            return binaryPath(options.installDir());
        } catch (Exception error) {
            throw new IllegalStateException("failed to install Wasmtime for " + runtimePlatform(), error);
        }
    }

    public static Path defaultInstallDir() {
        return Path.of(System.getProperty("user.home"), ".tikee", "wasmtime");
    }

    public static Path binaryPath(Path installDir) {
        return installDir.resolve("bin").resolve(isWindows() ? "wasmtime.exe" : "wasmtime");
    }

    public static String runtimePlatform() {
        return System.getProperty("os.name") + "/" + System.getProperty("os.arch");
    }

    public static boolean canUseOfficialInstaller() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        return !os.contains("windows") && commandAvailable("bash", Duration.ofSeconds(2))
                && commandAvailable("curl", Duration.ofSeconds(2));
    }

    private static boolean commandAvailable(String command, Duration timeout) {
        try {
            Process process = new ProcessBuilder(command, "--version")
                    .redirectErrorStream(true)
                    .start();
            if (!process.waitFor(timeout.toMillis(), TimeUnit.MILLISECONDS)) {
                process.destroyForcibly();
                return false;
            }
            return process.exitValue() == 0;
        } catch (Exception error) {
            return false;
        }
    }

    private static boolean isWindows() {
        return System.getProperty("os.name", "").toLowerCase(Locale.ROOT).contains("windows");
    }

    private static String shellQuote(String value) {
        String clean = value == null || value.isBlank() ? "latest" : value;
        return "'" + clean.replace("'", "'\\''") + "'";
    }
}
