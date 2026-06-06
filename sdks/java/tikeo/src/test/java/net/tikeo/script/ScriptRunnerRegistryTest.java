package net.tikeo.script;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import org.junit.jupiter.api.Test;

class ScriptRunnerRegistryTest {
    @Test
    void unavailableRunnerIsRegisteredForFailClosedExecutionButNotAdvertised() {
        ScriptRunnerRegistry registry = new ScriptRunnerRegistry();

        registry.register(new UnavailableScriptRunner(ScriptRunnerKind.PYTHON, "srt is not installed"));

        assertTrue(registry.find(ScriptRunnerKind.PYTHON).isPresent());
        assertTrue(registry.capabilities().isEmpty());
        assertTrue(registry.structuredCapabilities().isEmpty());
    }

    @Test
    void containerRuntimePathAdvertisesCanonicalSandboxBackend() {
        ScriptRunnerRegistry registry = new ScriptRunnerRegistry();

        registry.register(new ContainerScriptRunner(
                ScriptRunnerKind.SHELL,
                "/usr/bin/podman",
                "alpine:3.20",
                List.of("--pull=never")));

        assertEquals(1, registry.structuredCapabilities().size());
        assertEquals("podman", registry.structuredCapabilities().get(0).sandboxBackend());
    }
}
