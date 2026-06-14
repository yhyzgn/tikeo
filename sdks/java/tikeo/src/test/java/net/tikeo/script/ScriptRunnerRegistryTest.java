package net.tikeo.script;

import java.util.List;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;

class ScriptRunnerRegistryTest {
    @Test
    void unavailableRunnerIsRegisteredForFailClosedExecutionButNotAdvertised() {
        ScriptRunnerRegistry registry = new ScriptRunnerRegistry();

        registry.register(new UnavailableScriptRunner(ScriptRunnerKind.PYTHON, "srt is not installed"));

        Assertions.assertTrue(registry.find(ScriptRunnerKind.PYTHON).isPresent());
        Assertions.assertTrue(registry.capabilities().isEmpty());
        Assertions.assertTrue(registry.structuredCapabilities().isEmpty());
    }

    @Test
    void containerRuntimePathAdvertisesCanonicalSandboxBackend() {
        ScriptRunnerRegistry registry = new ScriptRunnerRegistry();

        registry.register(new ContainerScriptRunner(
                ScriptRunnerKind.SHELL,
                "/usr/bin/podman",
                "alpine:3.20",
                List.of("--pull=never")));

        Assertions.assertEquals(1, registry.structuredCapabilities().size());
        Assertions.assertEquals("podman", registry.structuredCapabilities().get(0).sandboxBackend());
    }
}
