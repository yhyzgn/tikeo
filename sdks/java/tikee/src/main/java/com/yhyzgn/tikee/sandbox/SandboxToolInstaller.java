package com.yhyzgn.tikee.sandbox;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.Locale;
import java.util.concurrent.TimeUnit;

/** Unified installer for worker-side sandbox/runtime tools. */
public final class SandboxToolInstaller {
    private SandboxToolInstaller() {}

    public enum Tool {
        WASMTIME("wasmtime"),
        WASMEDGE("wasmedge"),
        DENO("deno"),
        V8("v8"),
        RHAI("rhai-run");

        private final String binaryName;

        Tool(String binaryName) {
            this.binaryName = binaryName;
        }

        public String binaryName() {
            return binaryName;
        }
    }

    public record Options(
            Tool tool,
            String installVersion,
            Path installDir,
            String installerUrl,
            long installTimeoutMillis) {}

    public static Path install(Options options) {
        return switch (options.tool()) {
            case WASMTIME -> installWasmtime(options);
            case WASMEDGE -> installWasmEdge(options);
            case DENO -> installDeno(options);
            case V8 -> installV8(options);
            case RHAI -> installRhai(options);
        };
    }

    public static Path defaultInstallDir(Tool tool) {
        return Path.of(System.getProperty("user.home"), ".tikee", "sandbox-tools", tool.name().toLowerCase(Locale.ROOT));
    }

    public static Path binaryPath(Tool tool, Path installDir) {
        return installDir.resolve("bin").resolve(binaryName(tool));
    }

    public static String runtimePlatform() {
        return System.getProperty("os.name") + "/" + System.getProperty("os.arch");
    }

    public static boolean canInstall(Tool tool) {
        return switch (tool) {
            case WASMTIME -> canInstallWasmtime();
            case WASMEDGE -> canInstallWasmEdge();
            case DENO -> canInstallDeno();
            case V8 -> canInstallV8();
            case RHAI -> canInstallRhai();
        };
    }

    private static boolean canInstallWasmtime() {
        return canRunUnixInstaller() || commandAvailable("cargo", Duration.ofSeconds(2));
    }

    private static boolean canInstallWasmEdge() {
        return canRunUnixInstaller() || commandAvailable("winget", Duration.ofSeconds(2));
    }

    private static boolean canInstallDeno() {
        return canRunUnixInstaller() || commandAvailable("powershell", Duration.ofSeconds(2));
    }

    private static boolean canInstallV8() {
        return commandAvailable("cargo", Duration.ofSeconds(2));
    }

    private static boolean canInstallRhai() {
        return commandAvailable("cargo", Duration.ofSeconds(2));
    }

    private static Path installWasmtime(Options options) {
        if (canRunUnixInstaller()) {
            return runUnixWasmtimeInstaller(options);
        }
        return cargoInstall(options, "wasmtime-cli", "Wasmtime");
    }

    private static Path installWasmEdge(Options options) {
        if (canRunUnixInstaller()) {
            return runUnixWasmEdgeInstaller(options);
        }
        if (commandAvailable("winget", Duration.ofSeconds(2))) {
            runCommand(new ProcessBuilder("winget", "install", "-e", "--id", "WasmEdge.WasmEdge"), options.installTimeoutMillis(), "WasmEdge");
            return Path.of("wasmedge");
        }
        throw new IllegalStateException("no supported WasmEdge installer is available on " + runtimePlatform());
    }

    private static Path installDeno(Options options) {
        if (canRunUnixInstaller()) {
            try {
                Files.createDirectories(options.installDir());
                ProcessBuilder builder = new ProcessBuilder(
                        "bash",
                        "-c",
                        "curl -fsSL " + shellQuote(options.installerUrl()) + " | sh" + versionArg(options.installVersion()));
                builder.environment().put("DENO_INSTALL", options.installDir().toString());
                runCommand(builder, options.installTimeoutMillis(), "Deno");
                return binaryPath(options.tool(), options.installDir());
            } catch (Exception error) {
                throw new IllegalStateException("failed to install Deno for " + runtimePlatform(), error);
            }
        }
        runCommand(new ProcessBuilder("powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command",
                "irm " + options.installerUrl() + " | iex"), options.installTimeoutMillis(), "Deno");
        return Path.of("deno");
    }

    private static Path installV8(Options options) {
        return cargoInstall(options, "deno", "V8/Deno runtime");
    }

    private static Path installRhai(Options options) {
        return cargoInstall(options, "rhai", "Rhai");
    }

    private static Path runUnixWasmtimeInstaller(Options options) {
        try {
            Files.createDirectories(options.installDir());
            ProcessBuilder builder = new ProcessBuilder(
                    "bash",
                    "-c",
                    "curl -fsSL " + shellQuote(options.installerUrl()) + " | bash -s -- --version "
                            + shellQuote(options.installVersion()));
            builder.environment().put("WASMTIME_HOME", options.installDir().toString());
            builder.environment().put("PROFILE", "/dev/null");
            runCommand(builder, options.installTimeoutMillis(), "Wasmtime");
            return binaryPath(options.tool(), options.installDir());
        } catch (Exception error) {
            throw new IllegalStateException("failed to install Wasmtime for " + runtimePlatform(), error);
        }
    }

    private static Path runUnixWasmEdgeInstaller(Options options) {
        try {
            Files.createDirectories(options.installDir());
            StringBuilder command = new StringBuilder("curl -sSf ")
                    .append(shellQuote(options.installerUrl()))
                    .append(" | bash -s -- -p ")
                    .append(shellQuote(options.installDir().toString()));
            if (options.installVersion() != null && !options.installVersion().isBlank() && !"latest".equalsIgnoreCase(options.installVersion())) {
                command.append(" -v ").append(shellQuote(options.installVersion()));
            }
            runCommand(new ProcessBuilder("bash", "-c", command.toString()), options.installTimeoutMillis(), "WasmEdge");
            return binaryPath(options.tool(), options.installDir());
        } catch (Exception error) {
            throw new IllegalStateException("failed to install WasmEdge for " + runtimePlatform(), error);
        }
    }

    private static Path cargoInstall(Options options, String crateName, String label) {
        try {
            Files.createDirectories(options.installDir());
            java.util.ArrayList<String> command = new java.util.ArrayList<>();
            command.add("cargo");
            command.add("install");
            command.add("--root");
            command.add(options.installDir().toString());
            command.add(crateName);
            if (options.installVersion() != null && !options.installVersion().isBlank() && !"latest".equalsIgnoreCase(options.installVersion())) {
                command.add("--version");
                command.add(options.installVersion());
            }
            if (options.tool() == Tool.RHAI) {
                command.add("--bins");
                command.add("--features");
                command.add("bin-features");
            }
            runCommand(new ProcessBuilder(command), options.installTimeoutMillis(), label);
            return binaryPath(options.tool(), options.installDir());
        } catch (Exception error) {
            throw new IllegalStateException("failed to install " + label + " for " + runtimePlatform(), error);
        }
    }

    private static void runCommand(ProcessBuilder builder, long timeoutMillis, String label) {
        try {
            builder.redirectErrorStream(true);
            builder.redirectOutput(ProcessBuilder.Redirect.DISCARD);
            Process process = builder.start();
            if (!process.waitFor(timeoutMillis, TimeUnit.MILLISECONDS)) {
                process.destroyForcibly();
                throw new IllegalStateException(label + " install timed out after " + timeoutMillis + "ms");
            }
            if (process.exitValue() != 0) {
                throw new IllegalStateException(label + " installer exited with code " + process.exitValue());
            }
        } catch (Exception error) {
            throw new IllegalStateException(label + " installer failed", error);
        }
    }

    private static boolean canRunUnixInstaller() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        return !os.contains("windows")
                && commandAvailable("bash", Duration.ofSeconds(2))
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

    private static String binaryName(Tool tool) {
        String suffix = System.getProperty("os.name", "").toLowerCase(Locale.ROOT).contains("windows") ? ".exe" : "";
        return tool.binaryName() + suffix;
    }

    private static String versionArg(String version) {
        return version == null || version.isBlank() || "latest".equalsIgnoreCase(version) ? "" : " " + shellQuote(version);
    }

    private static String shellQuote(String value) {
        String clean = value == null || value.isBlank() ? "latest" : value;
        return "'" + clean.replace("'", "'\\''") + "'";
    }
}
