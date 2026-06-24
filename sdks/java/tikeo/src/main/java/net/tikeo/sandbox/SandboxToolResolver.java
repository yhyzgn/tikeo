package net.tikeo.sandbox;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Optional;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.TimeUnit;
import net.tikeo.script.ScriptRunnerKind;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Resolves sandbox/runtime tool commands and optionally installs missing tools.
 */
public final class SandboxToolResolver {

    private static final Logger log = LoggerFactory.getLogger(
        SandboxToolResolver.class
    );

    private static final java.util.Set<String> BACKGROUND_INSTALLS =
        ConcurrentHashMap.newKeySet();

    private final Options options;
    private final BackgroundInstaller backgroundInstaller;

    public SandboxToolResolver(Options options) {
        this(options, SandboxToolResolver::runBackgroundInstall);
    }

    SandboxToolResolver(Options options, BackgroundInstaller backgroundInstaller) {
        this.options = options == null ? Options.defaults() : options;
        this.backgroundInstaller = backgroundInstaller == null
            ? SandboxToolResolver::runBackgroundInstall
            : backgroundInstaller;
    }

    @FunctionalInterface
    interface BackgroundInstaller {
        void install(
            SandboxToolInstaller.Tool tool,
            SandboxToolInstaller.Options installOptions
        );
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
        if (!options.requireManagedTools() && toolAvailable(tool, command)) {
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
        String fallback = options.requireManagedTools() ? localCommand(tool) : tool.binaryName();
        log.info(
            "[tikeo.sandbox] tool={} unavailable after environment check; returning fallback command={}",
            tool.binaryName(),
            fallback
        );
        return fallback;
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
        String command = options.requireManagedTools()
            ? managedInterpreterCommand(binary)
            : binary;
        boolean available = "sh".equals(binary)
            ? runtimeAvailable(command, "-c", "exit 0")
            : runtimeAvailable(command, "--version");
        return available ? Optional.of(command) : Optional.empty();
    }

    private String managedInterpreterCommand(String binary) {
        return SandboxToolInstaller.defaultInstallDir(
            SandboxToolInstaller.Tool.SRT
        ).getParent().resolve(binary).resolve("bin").resolve(binary).toString();
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
        Optional<Path> legacyStateDir = legacyStateScopedInstallDir(tool);
        if (legacyStateDir.isPresent()) {
            return legacyStateDir.get();
        }
        return SandboxToolInstaller.defaultInstallDir(tool);
    }

    private Optional<Path> legacyStateScopedInstallDir(SandboxToolInstaller.Tool tool) {
        if (options.stateDir() == null || options.stateDir().isBlank()) {
            return Optional.empty();
        }
        Path installDir = Path.of(
            options.stateDir(),
            "sandbox-tools",
            installDirectoryKey(tool)
        );
        Path binary = SandboxToolInstaller.binaryPath(tool, installDir);
        return Files.isRegularFile(binary)
            ? Optional.of(installDir)
            : Optional.empty();
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
        ScriptRunnerKind kind
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
        Path script = null;
        try {
            script = Files.createTempFile(
                "tikeo-rhai-smoke-",
                ".rhai"
            );
            Files.writeString(script, "print(\"ok\");");
            return runtimeAvailable(command, script.toString());
        } catch (Exception error) {
            return false;
        } finally {
            if (script != null) {
                try {
                    Files.deleteIfExists(script);
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
            ArrayList<String> command = new ArrayList<>();
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
            return localCommand(tool);
        }
        scheduleBackgroundInstall(tool);
        return localCommand(tool);
    }

    private void scheduleBackgroundInstall(SandboxToolInstaller.Tool tool) {
        SandboxToolInstaller.Options installOptions = installOptions(tool);
        String key = tool.name() + "@" + installOptions.installDir().toAbsolutePath();
        if (!BACKGROUND_INSTALLS.add(key)) {
            log.debug(
                "[tikeo.sandbox] background install already scheduled tool={} installDir={}",
                tool.binaryName(),
                installOptions.installDir()
            );
            return;
        }
        Thread installer = new Thread(
            () -> backgroundInstaller.install(tool, installOptions),
            "tikeo-sandbox-install-" + tool.binaryName()
        );
        installer.setDaemon(true);
        installer.start();
        log.info(
            "[tikeo.sandbox] scheduled background install tool={} installDir={}",
            tool.binaryName(),
            installOptions.installDir()
        );
    }

    private static void runBackgroundInstall(
        SandboxToolInstaller.Tool tool,
        SandboxToolInstaller.Options installOptions
    ) {
        try {
            if (!SandboxToolInstaller.canInstall(tool)) {
                log.info(
                    "[tikeo.sandbox] background auto-install prerequisites missing tool={}",
                    tool.binaryName()
                );
                return;
            }
            Path binary = SandboxToolInstaller.install(installOptions);
            log.info(
                "[tikeo.sandbox] background auto-install completed tool={} binary={}",
                tool.binaryName(),
                binary
            );
        } catch (Exception error) {
            log.warn(
                "[tikeo.sandbox] background auto-install failed tool={} error={}",
                tool.binaryName(),
                error.getMessage()
            );
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
        boolean requireManagedTools,
        long installTimeoutMillis
    ) {
        public Options(
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
            this(
                stateDir,
                autoInstallWasmtime,
                wasmtimeInstallVersion,
                wasmtimeInstallDir,
                wasmtimeInstallerUrl,
                autoInstallWasmedge,
                wasmedgeInstallVersion,
                wasmedgeInstallDir,
                wasmedgeInstallerUrl,
                autoInstallScriptTools,
                srtInstallVersion,
                srtInstallDir,
                ripgrepInstallVersion,
                ripgrepInstallDir,
                denoInstallVersion,
                denoInstallDir,
                denoInstallerUrl,
                v8InstallVersion,
                v8InstallDir,
                rhaiInstallVersion,
                rhaiInstallDir,
                powerShellInstallVersion,
                powerShellInstallDir,
                false,
                installTimeoutMillis
            );
        }

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
                false,
                120_000
            );
        }
    }
}
