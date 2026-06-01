package com.yhyzgn.tikee.sandbox;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.yhyzgn.tikee.script.ScriptRunnerKind;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;

class SandboxToolResolverTest {
    @Test
    void resolvesStateScopedInstallDirectories() {
        SandboxToolResolver resolver = new SandboxToolResolver(new SandboxToolResolver.Options(
                "/tmp/tikee-state",
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
                "https://deno.land/install.sh",
                "latest",
                "",
                "",
                "",
                1000));

        assertEquals(Path.of("/tmp/tikee-state", "sandbox-tools", "srt"),
                resolver.installDir(SandboxToolInstaller.Tool.SRT));
        assertEquals(Path.of("/tmp/tikee-state", "sandbox-tools", "deno"),
                resolver.installDir(SandboxToolInstaller.Tool.DENO));
        assertEquals(Path.of("/tmp/tikee-state", "sandbox-tools", "wasmedge"),
                resolver.installDir(SandboxToolInstaller.Tool.WASMEDGE));
    }

    @Test
    void buildsJsTsCommandsWithResolvedDenoSandbox() {
        SandboxToolResolver resolver = new SandboxToolResolver(new SandboxToolResolver.Options(
                "/tmp/tikee-state",
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
                "/opt/tikee/deno",
                "https://deno.land/install.sh",
                "latest",
                "",
                "",
                "",
                1000));

        assertEquals(List.of("/opt/tikee/deno/bin/deno", "run", "--no-prompt", "-"),
                resolver.localDevelopmentCommand(ScriptRunnerKind.JS));
        assertEquals(List.of("/opt/tikee/deno/bin/deno", "run", "--no-prompt", "-"),
                resolver.localDevelopmentCommand(ScriptRunnerKind.TS));
    }

    @Test
    void resolvesManagedSandboxRuntimeMatrix() throws Exception {
        Path stateDir = java.nio.file.Files.createTempDirectory("tikee-sandbox-matrix-");
        installFake(stateDir, SandboxToolInstaller.Tool.WASMTIME);
        installFake(stateDir, SandboxToolInstaller.Tool.WASMEDGE);
        installFake(stateDir, SandboxToolInstaller.Tool.SRT);
        installFake(stateDir, SandboxToolInstaller.Tool.DENO);
        installFake(stateDir, SandboxToolInstaller.Tool.RHAI);
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
                stateDir.resolve("sandbox-tools/deno").toString(),
                "https://deno.land/install.sh",
                "latest",
                stateDir.resolve("sandbox-tools/v8").toString(),
                "",
                stateDir.resolve("sandbox-tools/rhai").toString(),
                1000));

        assertEquals(stateDir.resolve("sandbox-tools/wasmtime/bin/wasmtime").toString(),
                resolver.resolveWasmtimeCommand().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/wasmedge/bin/wasmedge").toString(),
                resolver.resolveWasmedgeCommand().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/srt/bin/srt").toString(),
                resolver.resolveSrtCommand().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/deno/bin/deno").toString(),
                resolver.resolveDenoCommand().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/deno/bin/deno").toString(),
                resolver.resolveV8Command().orElseThrow());
        assertEquals(stateDir.resolve("sandbox-tools/rhai/bin/rhai-run").toString(),
                resolver.resolveRhaiCommand().orElseThrow());
    }

    private static void installFake(Path stateDir, SandboxToolInstaller.Tool tool) throws Exception {
        Path binary = SandboxToolInstaller.binaryPath(tool,
                stateDir.resolve("sandbox-tools").resolve(tool.name().toLowerCase(java.util.Locale.ROOT)));
        java.nio.file.Files.createDirectories(binary.getParent());
        String body = "#!/usr/bin/env sh\n";
        if (tool == SandboxToolInstaller.Tool.RHAI) {
            body += "test -f \"$1\" && echo rhai-ok\n";
        } else {
            body += "echo " + tool.binaryName() + "-ok\n";
        }
        java.nio.file.Files.writeString(binary, body);
        binary.toFile().setExecutable(true);
    }

}
