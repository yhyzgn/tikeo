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
}
