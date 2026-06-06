package net.tikeo.sandbox;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.Locale;
import java.util.concurrent.TimeUnit;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/** Unified installer for worker-side sandbox/runtime tools. */
public final class SandboxToolInstaller {

    private static final Logger log = LoggerFactory.getLogger(
        SandboxToolInstaller.class
    );

    private SandboxToolInstaller() {}

    public enum Tool {
        WASMTIME("wasmtime"),
        WASMEDGE("wasmedge"),
        SRT("srt"),
        RIPGREP("rg"),
        DENO("deno"),
        V8("v8"),
        RHAI("rhai-run"),
        POWERSHELL("pwsh");

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
        long installTimeoutMillis
    ) {}

    public static Path install(Options options) {
        log.info(
            "[tikeo.sandbox] installing tool={} version={} installDir={} platform={} timeoutMs={}",
            options.tool().binaryName(),
            cleanVersion(options.installVersion()),
            options.installDir(),
            runtimePlatform(),
            options.installTimeoutMillis()
        );
        Path binary = switch (options.tool()) {
            case WASMTIME -> installWasmtime(options);
            case WASMEDGE -> installWasmEdge(options);
            case SRT -> installSrt(options);
            case RIPGREP -> installRipgrep(options);
            case DENO -> installDeno(options);
            case V8 -> installV8(options);
            case RHAI -> installRhai(options);
            case POWERSHELL -> installPowerShell(options);
        };
        log.info(
            "[tikeo.sandbox] installed tool={} binary={}",
            options.tool().binaryName(),
            binary
        );
        return binary;
    }

    public static Path defaultInstallDir(Tool tool) {
        return Path.of(
            System.getProperty("user.home"),
            ".tikeo",
            "sandbox-tools",
            tool.name().toLowerCase(Locale.ROOT)
        );
    }

    public static Path binaryPath(Tool tool, Path installDir) {
        return installDir.resolve("bin").resolve(binaryName(tool));
    }

    public static String runtimePlatform() {
        return (
            System.getProperty("os.name") + "/" + System.getProperty("os.arch")
        );
    }

    public static boolean canInstall(Tool tool) {
        boolean canInstall = switch (tool) {
            case WASMTIME -> canInstallWasmtime();
            case WASMEDGE -> canInstallWasmEdge();
            case SRT -> canInstallSrt();
            case RIPGREP -> canInstallRipgrep();
            case DENO -> canInstallDeno();
            case V8 -> canInstallV8();
            case RHAI -> canInstallRhai();
            case POWERSHELL -> canInstallPowerShell();
        };
        log.info(
            "[tikeo.sandbox] installer prerequisites tool={} available={}",
            tool.binaryName(),
            canInstall
        );
        return canInstall;
    }

    private static boolean canInstallWasmtime() {
        return (
            canRunUnixInstaller() ||
            commandAvailable("cargo", Duration.ofSeconds(2))
        );
    }

    private static boolean canInstallWasmEdge() {
        return (
            canRunUnixInstaller() ||
            commandAvailable("winget", Duration.ofSeconds(2))
        );
    }

    private static boolean canInstallDeno() {
        return (
            canRunUnixInstaller() ||
            commandAvailable("powershell", Duration.ofSeconds(2))
        );
    }

    private static boolean canInstallSrt() {
        return commandAvailable("npm", Duration.ofSeconds(2));
    }

    private static boolean canInstallRipgrep() {
        return commandAvailable("cargo", Duration.ofSeconds(2));
    }

    private static boolean canInstallV8() {
        return commandAvailable("cargo", Duration.ofSeconds(2));
    }

    private static boolean canInstallRhai() {
        return commandAvailable("cargo", Duration.ofSeconds(2));
    }

    private static boolean canInstallPowerShell() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        if (os.contains("windows")) {
            return commandAvailable("winget", Duration.ofSeconds(2));
        }
        return commandAvailable("curl", Duration.ofSeconds(2)) && commandAvailable("tar", Duration.ofSeconds(2));
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
            runCommand(
                new ProcessBuilder(
                    "winget",
                    "install",
                    "-e",
                    "--id",
                    "WasmEdge.WasmEdge"
                ),
                options.installTimeoutMillis(),
                "WasmEdge"
            );
            return Path.of("wasmedge");
        }
        throw new IllegalStateException(
            "no supported WasmEdge installer is available on " +
                runtimePlatform()
        );
    }

    private static Path installDeno(Options options) {
        if (canRunUnixInstaller()) {
            try {
                Files.createDirectories(options.installDir());
                ProcessBuilder builder = new ProcessBuilder(
                    "bash",
                    "-c",
                    "curl -fsSL " +
                        shellQuote(options.installerUrl()) +
                        " | sh" +
                        versionArg(options.installVersion())
                );
                builder
                    .environment()
                    .put("DENO_INSTALL", options.installDir().toString());
                runCommand(builder, options.installTimeoutMillis(), "Deno");
                return binaryPath(options.tool(), options.installDir());
            } catch (Exception error) {
                throw new IllegalStateException(
                    "failed to install Deno for " + runtimePlatform(),
                    error
                );
            }
        }
        runCommand(
            new ProcessBuilder(
                "powershell",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "irm " + options.installerUrl() + " | iex"
            ),
            options.installTimeoutMillis(),
            "Deno"
        );
        return Path.of("deno");
    }

    private static Path installSrt(Options options) {
        String packageName = "@anthropic-ai/sandbox-runtime";
        if (
            options.installVersion() != null &&
            !options.installVersion().isBlank() &&
            !"latest".equalsIgnoreCase(options.installVersion())
        ) {
            packageName += "@" + options.installVersion();
        }
        try {
            Files.createDirectories(options.installDir());
            runCommand(
                new ProcessBuilder(
                    "npm",
                    "install",
                    "-g",
                    "--prefix",
                    options.installDir().toString(),
                    packageName
                ),
                options.installTimeoutMillis(),
                "Anthropic Sandbox Runtime"
            );
            return binaryPath(options.tool(), options.installDir());
        } catch (Exception error) {
            throw new IllegalStateException(
                "failed to install Anthropic Sandbox Runtime for " + runtimePlatform(),
                error
            );
        }
    }

    private static Path installRipgrep(Options options) {
        return cargoInstall(options, "ripgrep", "ripgrep");
    }

    private static Path installV8(Options options) {
        return cargoInstall(options, "deno", "V8/Deno runtime");
    }

    private static Path installRhai(Options options) {
        return cargoInstall(options, "rhai", "Rhai");
    }

    private static Path installPowerShell(Options options) {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        if (os.contains("windows")) {
            runCommand(
                new ProcessBuilder("winget", "install", "-e", "--id", "Microsoft.PowerShell"),
                options.installTimeoutMillis(),
                "PowerShell"
            );
            return Path.of(binaryName(options.tool()));
        }
        String platform = powerShellArchivePlatform();
        if (platform.isBlank()) {
            throw new IllegalStateException("PowerShell auto-install is unsupported on " + runtimePlatform());
        }
        String version = System.getenv().getOrDefault("TIKEO_POWERSHELL_VERSION", "7.5.4");
        String archiveName = "powershell-" + version + "-" + platform + ".tar.gz";
        String url = System.getenv().getOrDefault(
            "TIKEO_POWERSHELL_DOWNLOAD_URL",
            "https://github.com/PowerShell/PowerShell/releases/download/v" + version + "/" + archiveName
        );
        Path archive = options.installDir().resolve(archiveName);
        Path extractDir = options.installDir().resolve("powershell-" + version);
        Path binDir = options.installDir().resolve("bin");
        try {
            Files.createDirectories(binDir);
            Files.createDirectories(extractDir);
            runCommand(
                new ProcessBuilder("curl", "-fsSL", url, "-o", archive.toString()),
                options.installTimeoutMillis(),
                "PowerShell download"
            );
            runCommand(
                new ProcessBuilder("tar", "-xzf", archive.toString(), "-C", extractDir.toString()),
                options.installTimeoutMillis(),
                "PowerShell extract"
            );
            Path pwsh = extractDir.resolve("pwsh");
            pwsh.toFile().setExecutable(true, false);
            Path link = binaryPath(options.tool(), options.installDir());
            Files.deleteIfExists(link);
            try {
                Files.createSymbolicLink(link, pwsh);
            } catch (Exception error) {
                Files.copy(pwsh, link, java.nio.file.StandardCopyOption.REPLACE_EXISTING);
                link.toFile().setExecutable(true, false);
            }
            Files.deleteIfExists(archive);
            return link;
        } catch (Exception error) {
            throw new IllegalStateException("failed to install PowerShell for " + runtimePlatform(), error);
        }
    }

    private static String powerShellArchivePlatform() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        String arch = System.getProperty("os.arch", "").toLowerCase(Locale.ROOT);
        if (os.contains("linux") && (arch.equals("amd64") || arch.equals("x86_64"))) {
            return "linux-x64";
        }
        if (os.contains("linux") && (arch.equals("aarch64") || arch.equals("arm64"))) {
            return "linux-arm64";
        }
        if ((os.contains("mac") || os.contains("darwin")) && (arch.equals("amd64") || arch.equals("x86_64"))) {
            return "osx-x64";
        }
        if ((os.contains("mac") || os.contains("darwin")) && (arch.equals("aarch64") || arch.equals("arm64"))) {
            return "osx-arm64";
        }
        return "";
    }

    private static Path runUnixWasmtimeInstaller(Options options) {
        try {
            Files.createDirectories(options.installDir());
            ProcessBuilder builder = new ProcessBuilder(
                "bash",
                "-c",
                "curl -fsSL " +
                    shellQuote(options.installerUrl()) +
                    " | bash -s -- --version " +
                    shellQuote(options.installVersion())
            );
            builder
                .environment()
                .put("WASMTIME_HOME", options.installDir().toString());
            builder.environment().put("PROFILE", "/dev/null");
            runCommand(builder, options.installTimeoutMillis(), "Wasmtime");
            return binaryPath(options.tool(), options.installDir());
        } catch (Exception error) {
            throw new IllegalStateException(
                "failed to install Wasmtime for " + runtimePlatform(),
                error
            );
        }
    }

    private static Path runUnixWasmEdgeInstaller(Options options) {
        try {
            Files.createDirectories(options.installDir());
            StringBuilder command = new StringBuilder("curl -sSf ")
                .append(shellQuote(options.installerUrl()))
                .append(" | bash -s -- -p ")
                .append(shellQuote(options.installDir().toString()));
            if (
                options.installVersion() != null &&
                !options.installVersion().isBlank() &&
                !"latest".equalsIgnoreCase(options.installVersion())
            ) {
                command
                    .append(" -v ")
                    .append(shellQuote(options.installVersion()));
            }
            runCommand(
                new ProcessBuilder("bash", "-c", command.toString()),
                options.installTimeoutMillis(),
                "WasmEdge"
            );
            return binaryPath(options.tool(), options.installDir());
        } catch (Exception error) {
            throw new IllegalStateException(
                "failed to install WasmEdge for " + runtimePlatform(),
                error
            );
        }
    }

    private static Path cargoInstall(
        Options options,
        String crateName,
        String label
    ) {
        try {
            Files.createDirectories(options.installDir());
            java.util.ArrayList<String> command = new java.util.ArrayList<>();
            command.add("cargo");
            command.add("install");
            command.add("--root");
            command.add(options.installDir().toString());
            command.add(crateName);
            if (
                options.installVersion() != null &&
                !options.installVersion().isBlank() &&
                !"latest".equalsIgnoreCase(options.installVersion())
            ) {
                command.add("--version");
                command.add(options.installVersion());
            }
            if (options.tool() == Tool.RHAI) {
                command.add("--bins");
                command.add("--features");
                command.add("bin-features");
            }
            runCommand(
                new ProcessBuilder(command),
                options.installTimeoutMillis(),
                label
            );
            return binaryPath(options.tool(), options.installDir());
        } catch (Exception error) {
            throw new IllegalStateException(
                "failed to install " + label + " for " + runtimePlatform(),
                error
            );
        }
    }

    private static void runCommand(
        ProcessBuilder builder,
        long timeoutMillis,
        String label
    ) {
        try {
            log.info(
                "[tikeo.sandbox] starting installer label={} command={}",
                label,
                sanitizedCommand(builder.command())
            );
            builder.redirectErrorStream(true);
            builder.redirectOutput(ProcessBuilder.Redirect.DISCARD);
            Process process = builder.start();
            if (!process.waitFor(timeoutMillis, TimeUnit.MILLISECONDS)) {
                process.destroyForcibly();
                throw new IllegalStateException(
                    label + " install timed out after " + timeoutMillis + "ms"
                );
            }
            if (process.exitValue() != 0) {
                throw new IllegalStateException(
                    label + " installer exited with code " + process.exitValue()
                );
            }
            log.info(
                "[tikeo.sandbox] installer finished label={} exitCode={}",
                label,
                process.exitValue()
            );
        } catch (Exception error) {
            throw new IllegalStateException(label + " installer failed", error);
        }
    }

    private static boolean canRunUnixInstaller() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        return (
            !os.contains("windows") &&
            commandAvailable("bash", Duration.ofSeconds(2)) &&
            commandAvailable("curl", Duration.ofSeconds(2))
        );
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
        String suffix = System.getProperty("os.name", "")
            .toLowerCase(Locale.ROOT)
            .contains("windows")
            ? ".exe"
            : "";
        return tool.binaryName() + suffix;
    }

    private static String versionArg(String version) {
        return version == null ||
            version.isBlank() ||
            "latest".equalsIgnoreCase(version)
            ? ""
            : " " + shellQuote(version);
    }

    private static java.util.List<String> sanitizedCommand(
        java.util.List<String> command
    ) {
        return command
            .stream()
            .map(part ->
                part == null
                    ? ""
                    : part.replaceAll(
                          "(?i)(token|password|secret)=\\S+",
                          "$1=<redacted>"
                      )
            )
            .toList();
    }

    private static String cleanVersion(String version) {
        return version == null || version.isBlank() ? "latest" : version;
    }

    private static String shellQuote(String value) {
        String clean = value == null || value.isBlank() ? "latest" : value;
        return "'" + clean.replace("'", "'\\''") + "'";
    }
}
