package com.yhyzgn.tikee.sandbox;

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
            "[tikee.sandbox] resolving tool={} platform={} installDir={}",
            tool.binaryName(),
            SandboxToolInstaller.runtimePlatform(),
            installDir(tool)
        );
        String command = tool.binaryName();
        if (
            explicitInstallDir(tool).isEmpty() && toolAvailable(tool, command)
        ) {
            log.info(
                "[tikee.sandbox] tool={} found on PATH command={}",
                tool.binaryName(),
                command
            );
            return command;
        }
        if (explicitInstallDir(tool).isEmpty()) {
            command = localCommand(tool);
            if (toolAvailable(tool, command)) {
                log.info(
                    "[tikee.sandbox] tool={} found in managed install command={}",
                    tool.binaryName(),
                    command
                );
                return command;
            }
        }
        command = installIfAllowed(tool);
        if (explicitInstallDir(tool).isPresent()) {
            if (toolAvailable(tool, command)) {
                log.info(
                    "[tikee.sandbox] tool={} installed/resolved command={}",
                    tool.binaryName(),
                    command
                );
                return command;
            }
            log.info(
                "[tikee.sandbox] tool={} unavailable after explicit-dir resolution; returning local command={}",
                tool.binaryName(),
                localCommand(tool)
            );
            return localCommand(tool);
        }
        if (toolAvailable(tool, command)) {
            log.info(
                "[tikee.sandbox] tool={} installed/resolved command={}",
                tool.binaryName(),
                command
            );
            return command;
        }
        log.info(
            "[tikee.sandbox] tool={} unavailable after environment check; returning fallback command={}",
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

    public Optional<String> resolveSrtCommand() {
        SandboxToolInstaller.Tool tool = SandboxToolInstaller.Tool.SRT;
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
                tool.name().toLowerCase(Locale.ROOT)
            );
        }
        return SandboxToolInstaller.defaultInstallDir(tool);
    }

    private Optional<Path> explicitInstallDir(SandboxToolInstaller.Tool tool) {
        String configured = switch (tool) {
            case WASMTIME -> options.wasmtimeInstallDir();
            case WASMEDGE -> options.wasmedgeInstallDir();
            case SRT -> options.srtInstallDir();
            case DENO -> options.denoInstallDir();
            case V8 -> options.v8InstallDir();
            case RHAI -> options.rhaiInstallDir();
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
        com.yhyzgn.tikee.script.ScriptRunnerKind kind
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
                "pwsh",
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
                "tikee-rhai-smoke-",
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
                "[tikee.sandbox] auto-install disabled tool={}",
                tool.binaryName()
            );
            return tool.binaryName();
        }
        if (!SandboxToolInstaller.canInstall(tool)) {
            log.info(
                "[tikee.sandbox] auto-install prerequisites missing tool={}",
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
                "[tikee.sandbox] auto-install failed tool={} error={}",
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
            case SRT, DENO, V8, RHAI -> options.autoInstallScriptTools();
        };
    }

    private String installVersion(SandboxToolInstaller.Tool tool) {
        return switch (tool) {
            case WASMTIME -> options.wasmtimeInstallVersion();
            case WASMEDGE -> options.wasmedgeInstallVersion();
            case SRT -> options.srtInstallVersion();
            case DENO -> options.denoInstallVersion();
            case V8 -> options.v8InstallVersion();
            case RHAI -> options.rhaiInstallVersion();
        };
    }

    private String installerUrl(SandboxToolInstaller.Tool tool) {
        return switch (tool) {
            case WASMTIME -> options.wasmtimeInstallerUrl();
            case WASMEDGE -> options.wasmedgeInstallerUrl();
            case SRT -> "";
            case DENO, V8 -> options.denoInstallerUrl();
            case RHAI -> "";
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
        String denoInstallVersion,
        String denoInstallDir,
        String denoInstallerUrl,
        String v8InstallVersion,
        String v8InstallDir,
        String rhaiInstallVersion,
        String rhaiInstallDir,
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
                "https://deno.land/install.sh",
                "latest",
                "",
                "",
                "",
                120_000
            );
        }
    }
}
