package net.tikeo.sandbox;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.nio.file.Path;
import org.junit.jupiter.api.Test;

class SandboxToolInstallerTest {
    @Test
    void binaryPathUsesInstallBinDirectory() {
        assertEquals(Path.of("/tmp/tikeo/wasmtime", "bin", "wasmtime"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.WASMTIME, Path.of("/tmp/tikeo/wasmtime")));
        assertEquals(Path.of("/tmp/tikeo/wasmedge", "bin", "wasmedge"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.WASMEDGE, Path.of("/tmp/tikeo/wasmedge")));
        assertEquals(Path.of("/tmp/tikeo/srt", "bin", "srt"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.SRT, Path.of("/tmp/tikeo/srt")));
        assertEquals(Path.of("/tmp/tikeo/ripgrep", "bin", "rg"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.RIPGREP, Path.of("/tmp/tikeo/ripgrep")));
        assertEquals(Path.of("/tmp/tikeo/deno", "bin", "deno"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.DENO, Path.of("/tmp/tikeo/deno")));
        assertEquals(Path.of("/tmp/tikeo/rhai", "bin", "rhai-run"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.RHAI, Path.of("/tmp/tikeo/rhai")));
        assertEquals(Path.of("/tmp/tikeo/pwsh", "bin", "pwsh"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.POWERSHELL, Path.of("/tmp/tikeo/pwsh")));
    }
}
