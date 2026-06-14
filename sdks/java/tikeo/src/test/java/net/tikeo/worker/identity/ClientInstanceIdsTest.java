package net.tikeo.worker.identity;

import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class ClientInstanceIdsTest {
    @TempDir
    Path stateRoot;

    @Test
    void explicitValueWinsWithoutWritingState() throws Exception {
        String id = ClientInstanceIds.resolve(" configured-id ", "default", "demo", "local", "local", stateRoot);

        Assertions.assertEquals("configured-id", id);
        Assertions.assertTrue(Files.notExists(stateRoot.resolve("default")));
    }

    @Test
    void generatedValueIsStableForSameScopeAndRuntimeIdentity() {
        String first = ClientInstanceIds.resolve(null, "default", "demo", "local", "local", stateRoot, "pod-a");
        String second = ClientInstanceIds.resolve(null, "default", "demo", "local", "local", stateRoot, "pod-a");

        Assertions.assertEquals(first, second);
        Assertions.assertTrue(first.startsWith("java-"));
    }

    @Test
    void generatedValueIsScopedByNamespaceAndApp() {
        String first = ClientInstanceIds.resolve(null, "default", "demo", "local", "local", stateRoot, "pod-a");
        String second = ClientInstanceIds.resolve(null, "default", "other", "local", "local", stateRoot, "pod-a");

        Assertions.assertNotEquals(first, second);
    }

    @Test
    void generatedValueIsScopedByRuntimeIdentity() {
        String first = ClientInstanceIds.resolve(null, "default", "demo", "local", "local", stateRoot, "pod-a");
        String second = ClientInstanceIds.resolve(null, "default", "demo", "local", "local", stateRoot, "pod-b");

        Assertions.assertNotEquals(first, second);
    }
}
