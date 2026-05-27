package com.yhyzgn.tikee.sandbox;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.nio.file.Path;
import org.junit.jupiter.api.Test;

class SandboxToolInstallerTest {
    @Test
    void binaryPathUsesInstallBinDirectory() {
        assertEquals(Path.of("/tmp/tikee/wasmtime", "bin", "wasmtime"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.WASMTIME, Path.of("/tmp/tikee/wasmtime")));
        assertEquals(Path.of("/tmp/tikee/wasmedge", "bin", "wasmedge"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.WASMEDGE, Path.of("/tmp/tikee/wasmedge")));
        assertEquals(Path.of("/tmp/tikee/srt", "bin", "srt"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.SRT, Path.of("/tmp/tikee/srt")));
        assertEquals(Path.of("/tmp/tikee/deno", "bin", "deno"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.DENO, Path.of("/tmp/tikee/deno")));
        assertEquals(Path.of("/tmp/tikee/rhai", "bin", "rhai-run"),
                SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.RHAI, Path.of("/tmp/tikee/rhai")));
    }
}
