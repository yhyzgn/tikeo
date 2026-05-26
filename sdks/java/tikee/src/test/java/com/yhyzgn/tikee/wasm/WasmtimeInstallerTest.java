package com.yhyzgn.tikee.wasm;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.nio.file.Path;
import org.junit.jupiter.api.Test;

class WasmtimeInstallerTest {
    @Test
    void binaryPathUsesInstallBinDirectory() {
        assertEquals(Path.of("/tmp/tikee/wasmtime", "bin", "wasmtime"),
                WasmtimeInstaller.binaryPath(Path.of("/tmp/tikee/wasmtime")));
    }
}
