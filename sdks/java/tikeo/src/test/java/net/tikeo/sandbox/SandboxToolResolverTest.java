package net.tikeo.sandbox;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Locale;
import net.tikeo.script.ScriptRunnerKind;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;

class SandboxToolResolverTest {
    @Test
    void resolvesHostScopedInstallDirectoriesWhenWorkerStateIsEmpty() {
        SandboxToolResolver resolver = new SandboxToolResolver(new SandboxToolResolver.Options(
                "/tmp/tikeo-state",
                false,
                "latest",
                "",
                "https://wasmtime.dev/install.sh",
                false,
                "latest",
                "",
                "https://wasmedge.example/install.sh",
                false,
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
                1000));

        Path hostCache = Path.of(System.getProperty("user.home"), ".tikeo", "sandbox-tools");
        Assertions.assertEquals(hostCache.resolve("srt"), resolver.installDir(SandboxToolInstaller.Tool.SRT));
        Assertions.assertEquals(hostCache.resolve("ripgrep"), resolver.installDir(SandboxToolInstaller.Tool.RIPGREP));
        Assertions.assertEquals(hostCache.resolve("deno"), resolver.installDir(SandboxToolInstaller.Tool.DENO));
        Assertions.assertEquals(hostCache.resolve("wasmedge"), resolver.installDir(SandboxToolInstaller.Tool.WASMEDGE));
        Assertions.assertEquals(hostCache.resolve("pwsh"), resolver.installDir(SandboxToolInstaller.Tool.POWERSHELL));
    }

    @Test
    void reusesLegacyStateScopedInstallWhenBinaryAlreadyExists() throws Exception {
        Path stateDir = Files.createTempDirectory("tikeo-legacy-sandbox-tools-");
        installFake(stateDir, SandboxToolInstaller.Tool.SRT);
        SandboxToolResolver resolver = new SandboxToolResolver(new SandboxToolResolver.Options(
                stateDir.toString(),
                false,
                "latest",
                "",
                "https://wasmtime.dev/install.sh",
                false,
                "latest",
                "",
                "https://wasmedge.example/install.sh",
                false,
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
                1000));

        Assertions.assertEquals(stateDir.resolve("sandbox-tools/srt"), resolver.installDir(SandboxToolInstaller.Tool.SRT));
        Assertions.assertEquals(stateDir.resolve("sandbox-tools/srt/bin/srt").toString(), resolver.resolveSrtCommand().orElseThrow());
    }

    @Test
    void buildsJsTsCommandsWithResolvedDenoSandbox() {
        SandboxToolResolver resolver = new SandboxToolResolver(new SandboxToolResolver.Options(
                "/tmp/tikeo-state",
                false,
                "latest",
                "",
                "https://wasmtime.dev/install.sh",
                false,
                "latest",
                "",
                "https://wasmedge.example/install.sh",
                false,
                "latest",
                "",
                "latest",
                "",
                "latest",
                "/opt/tikeo/deno",
                "https://deno.land/install.sh",
                "latest",
                "",
                "",
                "",
                "7.5.4",
                "",
                1000));

        Assertions.assertEquals(List.of("/opt/tikeo/deno/bin/deno", "run", "--no-prompt", "-"),
                resolver.localDevelopmentCommand(ScriptRunnerKind.JS));
        Assertions.assertEquals(List.of("/opt/tikeo/deno/bin/deno", "run", "--no-prompt", "-"),
                resolver.localDevelopmentCommand(ScriptRunnerKind.TS));
    }

    @Test
    void resolvesManagedSandboxRuntimeMatrix() throws Exception {
        Path stateDir = Files.createTempDirectory("tikeo-sandbox-matrix-");
        installFake(stateDir, SandboxToolInstaller.Tool.WASMTIME);
        installFake(stateDir, SandboxToolInstaller.Tool.WASMEDGE);
        installFake(stateDir, SandboxToolInstaller.Tool.SRT);
        installFake(stateDir, SandboxToolInstaller.Tool.RIPGREP);
        installFake(stateDir, SandboxToolInstaller.Tool.DENO);
        installFake(stateDir, SandboxToolInstaller.Tool.RHAI);
        installFake(stateDir, SandboxToolInstaller.Tool.POWERSHELL);
        SandboxToolResolver resolver = new SandboxToolResolver(new SandboxToolResolver.Options(
                stateDir.toString(),
                false,
                "latest",
                stateDir.resolve("sandbox-tools/wasmtime").toString(),
                "https://wasmtime.dev/install.sh",
                false,
                "latest",
                stateDir.resolve("sandbox-tools/wasmedge").toString(),
                "https://wasmedge.example/install.sh",
                false,
                "latest",
                stateDir.resolve("sandbox-tools/srt").toString(),
                "latest",
                stateDir.resolve("sandbox-tools/ripgrep").toString(),
                "latest",
                stateDir.resolve("sandbox-tools/deno").toString(),
                "https://deno.land/install.sh",
                "latest",
                stateDir.resolve("sandbox-tools/v8").toString(),
                "",
                stateDir.resolve("sandbox-tools/rhai").toString(),
                "7.5.4",
                stateDir.resolve("sandbox-tools/pwsh").toString(),
                1000));

        Assertions.assertEquals(stateDir.resolve("sandbox-tools/wasmtime/bin/wasmtime").toString(),
                resolver.resolveWasmtimeCommand().orElseThrow());
        Assertions.assertEquals(stateDir.resolve("sandbox-tools/wasmedge/bin/wasmedge").toString(),
                resolver.resolveWasmedgeCommand().orElseThrow());
        Assertions.assertEquals(stateDir.resolve("sandbox-tools/srt/bin/srt").toString(),
                resolver.resolveSrtCommand().orElseThrow());
        Assertions.assertEquals(stateDir.resolve("sandbox-tools/ripgrep/bin/rg").toString(),
                resolver.resolveRipgrepCommand().orElseThrow());
        Assertions.assertEquals(stateDir.resolve("sandbox-tools/deno/bin/deno").toString(),
                resolver.resolveDenoCommand().orElseThrow());
        Assertions.assertEquals(stateDir.resolve("sandbox-tools/deno/bin/deno").toString(),
                resolver.resolveV8Command().orElseThrow());
        Assertions.assertEquals(stateDir.resolve("sandbox-tools/rhai/bin/rhai-run").toString(),
                resolver.resolveRhaiCommand().orElseThrow());
        Assertions.assertEquals(stateDir.resolve("sandbox-tools/pwsh/bin/pwsh").toString(),
                resolver.resolvePowerShellCommand().orElseThrow());
    }


    @Test
    void autoInstallSchedulesBackgroundInstallWithoutBlockingResolution() throws Exception {
        Path toolsDir = Files.createTempDirectory("tikeo-async-sandbox-tools-");
        java.util.concurrent.atomic.AtomicBoolean scheduled = new java.util.concurrent.atomic.AtomicBoolean(false);
        SandboxToolResolver resolver = new SandboxToolResolver(new SandboxToolResolver.Options(
                "",
                false,
                "latest",
                "",
                "https://wasmtime.dev/install.sh",
                false,
                "latest",
                "",
                "https://wasmedge.example/install.sh",
                true,
                "latest",
                toolsDir.resolve("srt").toString(),
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
                1000),
            (tool, options) -> scheduled.set(true));

        long startedAt = System.nanoTime();
        Assertions.assertTrue(resolver.resolveSrtCommand().isEmpty());
        long elapsedMillis = java.util.concurrent.TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - startedAt);

        Assertions.assertTrue(scheduled.get(), "missing tool should schedule background install");
        Assertions.assertTrue(elapsedMillis < 1_000, "resolution must not wait for installer; elapsedMs=" + elapsedMillis);
    }

    private static void installFake(Path stateDir, SandboxToolInstaller.Tool tool) throws Exception {
        String installKey = tool == SandboxToolInstaller.Tool.POWERSHELL
                ? "pwsh"
                : tool.name().toLowerCase(Locale.ROOT);
        Path binary = SandboxToolInstaller.binaryPath(tool,
                stateDir.resolve("sandbox-tools").resolve(installKey));
        Files.createDirectories(binary.getParent());
        String body = "#!/usr/bin/env sh\n";
        if (tool == SandboxToolInstaller.Tool.RHAI) {
            body += "case \"${1:-}\" in\n";
            body += "  \"\"|\"--version\"|\"-V\") echo rhai-ok; exit 0 ;;\n";
            body += "esac\n";
            body += "test -f \"${1:-}\" && echo rhai-ok\n";
        } else {
            body += "echo " + tool.binaryName() + "-ok\n";
        }
        Files.writeString(binary, body);
        binary.toFile().setExecutable(true);
    }

}
