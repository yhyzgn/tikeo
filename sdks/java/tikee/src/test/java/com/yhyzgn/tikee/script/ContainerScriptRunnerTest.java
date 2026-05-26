package com.yhyzgn.tikee.script;

import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.List;
import org.junit.jupiter.api.Test;

class ContainerScriptRunnerTest {
    @Test
    void buildsSandboxedDockerCommandForShellScripts() throws Exception {
        ContainerScriptRunner runner = new ContainerScriptRunner(
                ScriptRunnerKind.SHELL,
                "docker",
                "alpine:3.20",
                List.of("--cpus=1"));

        List<String> command = runner.command(task("echo hello", policy(false, List.of(), List.of())));

        assertTrue(command.contains("--network=none"));
        assertTrue(command.contains("--read-only"));
        assertTrue(command.contains("/tmp:rw,noexec,nosuid,size=16m"));
        assertTrue(command.contains("--memory=1048576"));
        assertTrue(command.contains("--cpus=1"));
        assertTrue(command.contains("alpine:3.20"));
        assertTrue(command.subList(command.size() - 2, command.size()).equals(List.of("sh", "-s")));
    }

    @Test
    void rejectsDigestMismatchBeforeStartingContainer() {
        ContainerScriptRunner runner = new ContainerScriptRunner(ScriptRunnerKind.SHELL, "alpine:3.20");
        ScriptRunnerTask task = new ScriptRunnerTask(
                "script-1",
                "sv-1",
                1,
                "shell",
                "echo hello",
                "bad-digest",
                policy(false, List.of(), List.of()));

        assertThrows(ScriptRunnerException.class, () -> runner.command(task));
    }

    @Test
    void rejectsNetworkAndSecretGrantsUnlessDedicatedSandboxCanEnforceThem() throws Exception {
        ContainerScriptRunner runner = new ContainerScriptRunner(ScriptRunnerKind.SHELL, "alpine:3.20");

        assertThrows(ScriptRunnerException.class,
                () -> runner.command(task("echo hello", policy(true, List.of(), List.of()))));
        assertThrows(ScriptRunnerException.class,
                () -> runner.command(task("echo hello", policy(false, List.of("secret:db"), List.of()))));
    }

    @Test
    void mountsOnlyCleanAbsoluteFileGrants() throws Exception {
        ContainerScriptRunner runner = new ContainerScriptRunner(ScriptRunnerKind.SHELL, "alpine:3.20");

        assertTrue(runner.command(task("cat /data/input", policy(false, List.of(), List.of("/data/input"))))
                .contains("type=bind,src=/data/input,dst=/data/input,readonly"));
        assertThrows(ScriptRunnerException.class,
                () -> runner.command(task("cat x", policy(false, List.of(), List.of("../bad")))));
    }

    private static ScriptRunnerTask task(String content, ScriptRunnerPolicy policy) throws Exception {
        return new ScriptRunnerTask("script-1", "sv-1", 1, "shell", content, sha256(content), policy);
    }

    private static ScriptRunnerPolicy policy(boolean allowNetwork, List<String> secrets, List<String> readOnlyPaths) {
        return new ScriptRunnerPolicy(
                1000,
                1048576,
                1048576,
                allowNetwork,
                List.of(),
                List.of(),
                readOnlyPaths,
                List.of(),
                secrets);
    }

    private static String sha256(String content) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256")
                .digest(content.getBytes(StandardCharsets.UTF_8)));
    }
}
