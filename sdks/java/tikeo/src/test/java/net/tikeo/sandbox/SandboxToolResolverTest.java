package net.tikeo.sandbox;

import static org.junit.jupiter.api.Assertions.assertEquals;

import net.tikeo.script.ScriptRunnerKind;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;

class SandboxToolResolverTest {
    @Test
    void resolvesStateScopedInstallDirectories() {
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

        assertEquals(Path.of("/tmp/tikeo-state", "sandbox-tools", "srt"),
                resolver.installDir(SandboxToolInstaller.Tool.SRT));
        assertEquals(Path.of("/tmp/tikeo-state", "sandbox-tools", "ripgrep"),
                resolver.installDir(SandboxToolInstaller.Tool.RIPGREP));
        assertEquals(Path.of("/tmp/tikeo-state", "sandbox-tools", "deno"),
                resolver.installDir(SandboxToolInstaller.Tool.DENO));
        assertEquals(Path.of("/tmp/tikeo-state", "sandbox-tools", "wasmedge"),
                resolver.installDir(SandboxToolInstaller.Tool.WASMEDGE));
        assertEquals(Path.of("/tmp/tikeo-state", "sandbox-tools", "pwsh"),
                resolver.installDir(SandboxToolInstaller.Tool.POWERSHELL));
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

        assertEquals(List.of("/opt/tikeo/deno/bin/deno", "run", "--no-prompt", "-"),
                resolver.localDevelopmentCommand(ScriptRunnerKind.JS));
        assertEquals(List.of("/opt/tikeo/deno/bin/deno", "run", "--no-prompt", "-"),
                resolver.localDevelopmentCommand(ScriptRunnerKind.TS));
    }

    @Test
    void resolvesManagedSandboxRuntimeMatrix() throws Exception {
        Path stateDir = java.nio.file.Files.createTempDirectory("tikeo-sandbox-matrix-");
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

        assertEquals(stateDir.resolve("sandbox-tools/wasmtime/bin/wasmtime").toString(),
                resolver.resolveWasmtimeCommand().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/wasmedge/bin/wasmedge").toString(),
                resolver.resolveWasmedgeCommand().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/srt/bin/srt").toString(),
                resolver.resolveSrtCommand().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/ripgrep/bin/rg").toString(),
                resolver.resolveRipgrepCommand().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/deno/bin/deno").toString(),
                resolver.resolveDenoCommand().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/deno/bin/deno").toString(),
                resolver.resolveV8Command().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/rhai/bin/rhai-run").toString(),
                resolver.resolveRhaiCommand().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/pwsh/bin/pwsh").toString(),
                resolver.resolvePowerShellCommand().orElseThrow());
    }

    private static void installFake(Path stateDir, SandboxToolInstaller.Tool tool) throws Exception {
        String installKey = tool == SandboxToolInstaller.Tool.POWERSHELL
                ? "pwsh"
                : tool.name().toLowerCase(java.util.Locale.ROOT);
        Path binary = SandboxToolInstaller.binaryPath(tool,
                stateDir.resolve("sandbox-tools").resolve(installKey));
        java.nio.file.Files.createDirectories(binary.getParent());
        String body = "#!/usr/bin/env sh\n";
        if (tool == SandboxToolInstaller.Tool.RHAI) {
            body += "case \"${1:-}\" in\n";
            body += "  \"\"|\"--version\"|\"-V\") echo rhai-ok; exit 0 ;;\n";
            body += "esac\n";
            body += "test -f \"${1:-}\" && echo rhai-ok\n";
        } else {
            body += "echo " + tool.binaryName() + "-ok\n";
        }
        java.nio.file.Files.writeString(binary, body);
        binary.toFile().setExecutable(true);
    }

}
