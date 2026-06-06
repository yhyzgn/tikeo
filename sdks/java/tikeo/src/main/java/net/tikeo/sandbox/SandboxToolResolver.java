package net.tikeo.sandbox;

import java.nio.file.Path;
import java.util.List;
import java.util.Locale;
import java.util.Optional;
import java.util.concurrent.TimeUnit;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/** Resolves sandbox/runtime tool commands and optionally installs missing tools. */
public final class SandboxToolResolver {

    private static final Logger log = LoggerFactory.getLogger(
        SandboxToolResolver.class
    );

    private final Options options;

    public SandboxToolResolver(Options options) {
        this.options = options == null ? Options.defaults() : options;
    }

    public String resolveCommand(SandboxToolInstaller.Tool tool) {
        log.info(
            "[tikeo.sandbox] resolving tool={} platform={} installDir={}",
            tool.binaryName(),
            SandboxToolInstaller.runtimePlatform(),
            installDir(tool)
        );
        Optional<Path> explicitInstallDir = explicitInstallDir(tool);
        if (explicitInstallDir.isPresent()) {
            String command = localCommand(tool);
            if (toolAvailable(tool, command)) {
                log.info(
                    "[tikeo.sandbox] tool={} found in explicit install command={}",
                    tool.binaryName(),
                    command
                );
                return command;
            }
            command = installIfAllowed(tool);
            if (toolAvailable(tool, command)) {
                log.info(
                    "[tikeo.sandbox] tool={} installed/resolved command={}",
                    tool.binaryName(),
                    command
                );
                return command;
            }
            log.info(
                "[tikeo.sandbox] tool={} unavailable after explicit-dir resolution; returning local command={}",
                tool.binaryName(),
                localCommand(tool)
            );
            return localCommand(tool);
        }

        String command = tool.binaryName();
        if (toolAvailable(tool, command)) {
            log.info(
                "[tikeo.sandbox] tool={} found on PATH command={}",
                tool.binaryName(),
                command
            );
            return command;
        }
        command = localCommand(tool);
        if (toolAvailable(tool, command)) {
            log.info(
                "[tikeo.sandbox] tool={} found in managed install command={}",
                tool.binaryName(),
                command
            );
            return command;
        }
        command = installIfAllowed(tool);
        if (toolAvailable(tool, command)) {
            log.info(
                "[tikeo.sandbox] tool={} installed/resolved command={}",
                tool.binaryName(),
                command
            );
            return command;
        }
        log.info(
            "[tikeo.sandbox] tool={} unavailable after environment check; returning fallback command={}",
            tool.binaryName(),
            tool.binaryName()
        );
        return tool.binaryName();
    }

    public Optional<String> resolveWasmtimeCommand() {
        SandboxToolInstaller.Tool tool = SandboxToolInstaller.Tool.WASMTIME;
        String command = resolveCommand(tool);
        return toolAvailable(tool, command)
            ? Optional.of(command)
            : Optional.empty();
    }

    public Optional<String> resolveWasmedgeCommand() {
        SandboxToolInstaller.Tool tool = SandboxToolInstaller.Tool.WASMEDGE;
        String command = resolveCommand(tool);
        return toolAvailable(tool, command)
            ? Optional.of(command)
            : Optional.empty();
    }

    public Optional<String> resolveSrtCommand() {
        SandboxToolInstaller.Tool tool = SandboxToolInstaller.Tool.SRT;
        String command = resolveCommand(tool);
        return toolAvailable(tool, command)
            ? Optional.of(command)
            : Optional.empty();
    }

    public Optional<String> resolveNodeCommand() {
        return resolveInterpreterCommand("node");
    }

    public Optional<String> resolveNpmCommand() {
        return resolveInterpreterCommand("npm");
    }

    public Optional<String> resolveRipgrepCommand() {
        SandboxToolInstaller.Tool tool = SandboxToolInstaller.Tool.RIPGREP;
        String command = resolveCommand(tool);
        return toolAvailable(tool, command)
            ? Optional.of(command)
            : Optional.empty();
    }

    public Optional<String> resolveDenoCommand() {
        SandboxToolInstaller.Tool tool = SandboxToolInstaller.Tool.DENO;
        String command = resolveCommand(tool);
        return toolAvailable(tool, command)
            ? Optional.of(command)
            : Optional.empty();
    }

    public Optional<String> resolveV8Command() {
        // Tikeo's JavaScript/TypeScript V8 backend is currently fulfilled by
        // the Deno runtime, which embeds V8 and supplies the permission sandbox.
        return resolveDenoCommand();
    }

    public Optional<String> resolvePowerShellCommand() {
        SandboxToolInstaller.Tool tool = SandboxToolInstaller.Tool.POWERSHELL;
        String command = resolveCommand(tool);
        return toolAvailable(tool, command)
            ? Optional.of(command)
            : Optional.empty();
    }

    public Optional<String> resolveInterpreterCommand(String binary) {
        boolean available = "sh".equals(binary)
            ? runtimeAvailable(binary, "-c", "exit 0")
            : runtimeAvailable(binary, "--version");
        return available ? Optional.of(binary) : Optional.empty();
    }

    public Optional<String> resolveRhaiCommand() {
        SandboxToolInstaller.Tool tool = SandboxToolInstaller.Tool.RHAI;
        String command = resolveCommand(tool);
        return toolAvailable(tool, command)
            ? Optional.of(command)
            : Optional.empty();
    }

    public String localCommand(SandboxToolInstaller.Tool tool) {
        return SandboxToolInstaller.binaryPath(
            tool,
            installDir(tool)
        ).toString();
    }

    public Path installDir(SandboxToolInstaller.Tool tool) {
        Optional<Path> explicit = explicitInstallDir(tool);
        if (explicit.isPresent()) {
            return explicit.get();
        }
        if (options.stateDir() != null && !options.stateDir().isBlank()) {
            return Path.of(
                options.stateDir(),
                "sandbox-tools",
                installDirectoryKey(tool)
            );
        }
        return SandboxToolInstaller.defaultInstallDir(tool);
    }

    private static String installDirectoryKey(SandboxToolInstaller.Tool tool) {
        return tool == SandboxToolInstaller.Tool.POWERSHELL
            ? "pwsh"
            : tool.name().toLowerCase(Locale.ROOT);
    }

    private Optional<Path> explicitInstallDir(SandboxToolInstaller.Tool tool) {
        String configured = switch (tool) {
            case WASMTIME -> options.wasmtimeInstallDir();
            case WASMEDGE -> options.wasmedgeInstallDir();
            case SRT -> options.srtInstallDir();
            case RIPGREP -> options.ripgrepInstallDir();
            case DENO -> options.denoInstallDir();
            case V8 -> options.v8InstallDir();
            case RHAI -> options.rhaiInstallDir();
            case POWERSHELL -> options.powerShellInstallDir();
        };
        return configured == null || configured.isBlank()
            ? Optional.empty()
            : Optional.of(Path.of(configured));
    }

    public SandboxToolInstaller.Options installOptions(
        SandboxToolInstaller.Tool tool
    ) {
        return new SandboxToolInstaller.Options(
            tool,
            installVersion(tool),
            installDir(tool),
            installerUrl(tool),
            options.installTimeoutMillis()
        );
    }

    public List<String> localDevelopmentCommand(
        net.tikeo.script.ScriptRunnerKind kind
    ) {
        return switch (kind) {
            case SHELL -> List.of("sh", "-s");
            case PYTHON -> List.of("python3", "-");
            case JS, TS -> List.of(
                resolveCommand(SandboxToolInstaller.Tool.DENO),
                "run",
                "--no-prompt",
                "-"
            );
            case POWERSHELL -> List.of(
                resolveCommand(SandboxToolInstaller.Tool.POWERSHELL),
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "-"
            );
            case PHP -> List.of("php");
            case GROOVY -> List.of("groovy");
            case RHAI -> List.of(
                resolveCommand(SandboxToolInstaller.Tool.RHAI)
            );
        };
    }

    public static boolean toolAvailable(
        SandboxToolInstaller.Tool tool,
        String command
    ) {
        return switch (tool) {
            case SRT -> srtAvailable(command);
            case RHAI -> rhaiAvailable(command);
            case RIPGREP -> runtimeAvailable(command, "--version");
            default -> runtimeAvailable(command, "--version");
        };
    }

    private static boolean srtAvailable(String command) {
        return runtimeAvailable(command, "--version") ||
            runtimeAvailable(command, "--help");
    }

    private static boolean rhaiAvailable(String command) {
        java.nio.file.Path script = null;
        try {
            script = java.nio.file.Files.createTempFile(
                "tikeo-rhai-smoke-",
                ".rhai"
            );
            java.nio.file.Files.writeString(script, "print(\"ok\");");
            return runtimeAvailable(command, script.toString());
        } catch (Exception error) {
            return false;
        } finally {
            if (script != null) {
                try {
                    java.nio.file.Files.deleteIfExists(script);
                } catch (Exception ignored) {
                    // Smoke-test cleanup failure does not affect availability.
                }
            }
        }
    }

    public static boolean runtimeAvailable(
        String runtimeCommand,
        String... args
    ) {
        try {
            java.util.ArrayList<String> command = new java.util.ArrayList<>();
            command.add(runtimeCommand);
            command.addAll(List.of(args));
            Process process = new ProcessBuilder(command)
                .redirectErrorStream(true)
                .start();
            if (!process.waitFor(2, TimeUnit.SECONDS)) {
                process.destroyForcibly();
                return false;
            }
            return process.exitValue() == 0;
        } catch (Exception error) {
            return false;
        }
    }

    private String installIfAllowed(SandboxToolInstaller.Tool tool) {
        if (!autoInstallEnabled(tool)) {
            log.info(
                "[tikeo.sandbox] auto-install disabled tool={}",
                tool.binaryName()
            );
            return tool.binaryName();
        }
        if (!SandboxToolInstaller.canInstall(tool)) {
            log.info(
                "[tikeo.sandbox] auto-install prerequisites missing tool={}",
                tool.binaryName()
            );
            return tool.binaryName();
        }
        try {
            return SandboxToolInstaller.install(
                installOptions(tool)
            ).toString();
        } catch (IllegalStateException error) {
            log.info(
                "[tikeo.sandbox] auto-install failed tool={} error={}",
                tool.binaryName(),
                error.getMessage()
            );
            return tool.binaryName();
        }
    }

    private boolean autoInstallEnabled(SandboxToolInstaller.Tool tool) {
        return switch (tool) {
            case WASMTIME -> options.autoInstallWasmtime();
            case WASMEDGE -> options.autoInstallWasmedge();
            case SRT, RIPGREP, DENO, V8, RHAI, POWERSHELL -> options.autoInstallScriptTools();
        };
    }

    private String installVersion(SandboxToolInstaller.Tool tool) {
        return switch (tool) {
            case WASMTIME -> options.wasmtimeInstallVersion();
            case WASMEDGE -> options.wasmedgeInstallVersion();
            case SRT -> options.srtInstallVersion();
            case RIPGREP -> options.ripgrepInstallVersion();
            case DENO -> options.denoInstallVersion();
            case V8 -> options.v8InstallVersion();
            case RHAI -> options.rhaiInstallVersion();
            case POWERSHELL -> options.powerShellInstallVersion();
        };
    }

    private String installerUrl(SandboxToolInstaller.Tool tool) {
        return switch (tool) {
            case WASMTIME -> options.wasmtimeInstallerUrl();
            case WASMEDGE -> options.wasmedgeInstallerUrl();
            case SRT, RIPGREP -> "";
            case DENO, V8 -> options.denoInstallerUrl();
            case RHAI, POWERSHELL -> "";
        };
    }

    public record Options(
        String stateDir,
        boolean autoInstallWasmtime,
        String wasmtimeInstallVersion,
        String wasmtimeInstallDir,
        String wasmtimeInstallerUrl,
        boolean autoInstallWasmedge,
        String wasmedgeInstallVersion,
        String wasmedgeInstallDir,
        String wasmedgeInstallerUrl,
        boolean autoInstallScriptTools,
        String srtInstallVersion,
        String srtInstallDir,
        String ripgrepInstallVersion,
        String ripgrepInstallDir,
        String denoInstallVersion,
        String denoInstallDir,
        String denoInstallerUrl,
        String v8InstallVersion,
        String v8InstallDir,
        String rhaiInstallVersion,
        String rhaiInstallDir,
        String powerShellInstallVersion,
        String powerShellInstallDir,
        long installTimeoutMillis
    ) {
        public static Options defaults() {
            return new Options(
                "",
                true,
                "latest",
                "",
                "https://wasmtime.dev/install.sh",
                false,
                "latest",
                "",
                "https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh",
                true,
                "latest",
                "",
                "latest",
                "",
                "latest",
                "",
                "https://deno.land/install.sh",
                "latest",
                "",
                "",
                "",
                "7.5.4",
                "",
                120_000
            );
        }
    }
}
