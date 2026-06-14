package net.tikeo.sandbox;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;

class SandboxToolInstallerTest {
    @Test
    void powerShellInstallerReusesPublishedBinaryWithoutDownloadingAgain() throws Exception {
        Path installDir = Files.createTempDirectory("tikeo-pwsh-complete-install-");
        Path binary = SandboxToolInstaller.binaryPath(SandboxToolInstaller.Tool.POWERSHELL, installDir);
        Files.createDirectories(binary.getParent());
        Files.writeString(binary, "#!/usr/bin/env sh\necho PowerShell 7.5.4\n");
        binary.toFile().setExecutable(true);

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
                installDir.toString(),
                1));

        Assertions.assertTrue(resolver.resolvePowerShellCommand().isPresent());
        Assertions.assertTrue(Files.exists(binary), "published binary should remain the resolved command");
    }
    @Test
    void powerShellInstallerUsesExistingArchiveBeforeDownloading() throws Exception {
        Path sourceRoot = Files.createTempDirectory("tikeo-pwsh-source-");
        Path sourcePwsh = sourceRoot.resolve("pwsh");
        Files.writeString(sourcePwsh, "#!/usr/bin/env sh\necho PowerShell 7.5.4\n");
        sourcePwsh.toFile().setExecutable(true);
        Path archive = Files.createTempDirectory("tikeo-pwsh-archive-").resolve("powershell-7.5.4-linux-x64.tar.gz");
        Process tar = new ProcessBuilder("tar", "-czf", archive.toString(), "-C", sourceRoot.toString(), "pwsh")
                .inheritIO()
                .start();
        Assertions.assertTrue(tar.waitFor(5, TimeUnit.SECONDS));
        Assertions.assertEquals(0, tar.exitValue());

        Path installDir = Files.createTempDirectory("tikeo-pwsh-existing-archive-");
        Files.copy(archive, installDir.resolve("powershell-7.5.4-linux-x64.tar.gz"));

        Path installed = SandboxToolInstaller.install(new SandboxToolInstaller.Options(
                SandboxToolInstaller.Tool.POWERSHELL,
                "7.5.4",
                installDir,
                "",
                1));

        Assertions.assertTrue(Files.isRegularFile(installed), "existing archive should publish bin/pwsh");
        Assertions.assertFalse(Files.exists(installDir.resolve("powershell-7.5.4-linux-x64.tar.gz.part")),
                "existing complete archive must not trigger resumable download state");
    }

}
