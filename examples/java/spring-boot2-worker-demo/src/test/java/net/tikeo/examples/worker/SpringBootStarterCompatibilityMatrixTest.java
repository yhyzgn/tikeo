package net.tikeo.examples.worker;

import static org.assertj.core.api.Assertions.assertThat;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;

/** Verifies this standalone demo uses its matching Spring Boot starter artifact. */
class SpringBootStarterCompatibilityMatrixTest {
    private static final Path DEMO_ROOT = Path.of("").toAbsolutePath();
    private static final Path REPO_ROOT = DEMO_ROOT.resolve("../../..").normalize();
    private static final Path JAVA_SDK_ROOT = REPO_ROOT.resolve("sdks/java");

    @Test
    void demoUsesMatchingSpringBootStarter() throws IOException {
        String build = Files.readString(DEMO_ROOT.resolve("build.gradle.kts"));
        String readme = Files.readString(DEMO_ROOT.resolve("README.md"));

        assertThat(build)
                .contains("2.7.18")
                .contains("net.tikeo:tikeo-spring-boot2-starter:0.1.0-SNAPSHOT");
        assertThat(readme)
                .contains("Spring Boot 2.x")
                .contains("tikeo-spring-boot2-starter");
    }

    @Test
    void sdkPublishesSeparateBoot2Boot3AndBoot4StarterModulesWithRealSources() throws IOException {
        String settings = Files.readString(JAVA_SDK_ROOT.resolve("settings.gradle.kts"));
        String sdkReadme = Files.readString(JAVA_SDK_ROOT.resolve("README.md"));

        assertThat(settings)
                .contains("tikeo-spring-boot-starter")
                .contains("tikeo-spring-boot2-starter")
                .contains("tikeo-spring-boot3-starter")
                .contains("tikeo-spring5")
                .contains("tikeo-spring6");
        assertThat(sdkReadme)
                .contains("primary Spring Boot 4.x starter")
                .contains("Spring Boot 2.x projects")
                .contains("Spring Boot 3.x projects");

        for (String module : List.of(
                "tikeo-spring-boot-starter",
                "tikeo-spring-boot2-starter",
                "tikeo-spring-boot3-starter")) {
            assertRealStarterModule(module);
        }
        for (String module : List.of("tikeo-spring", "tikeo-spring5", "tikeo-spring6")) {
            assertRealSpringAdapterModule(module);
        }
    }

    private static void assertRealStarterModule(String module) throws IOException {
        Path moduleRoot = JAVA_SDK_ROOT.resolve(module);
        assertThat(countJavaSources(moduleRoot.resolve("src/main/java")))
                .as(module + " main Java source count")
                .isGreaterThanOrEqualTo(4);
        assertThat(countJavaSources(moduleRoot.resolve("src/test/java")))
                .as(module + " test Java source count")
                .isGreaterThanOrEqualTo(1);
        assertThat(moduleRoot.resolve("src/main/resources/META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports"))
                .as(module + " Boot 2.7+/3/4 metadata")
                .exists();
        if (module.endsWith("boot2-starter")) {
            assertThat(moduleRoot.resolve("src/main/resources/META-INF/spring.factories"))
                    .as(module + " Boot 2 metadata")
                    .exists();
        }
    }

    private static void assertRealSpringAdapterModule(String module) throws IOException {
        Path moduleRoot = JAVA_SDK_ROOT.resolve(module);
        assertThat(countJavaSources(moduleRoot.resolve("src/main/java")))
                .as(module + " main Java source count")
                .isGreaterThanOrEqualTo(4);
        assertThat(countJavaSources(moduleRoot.resolve("src/test/java")))
                .as(module + " test Java source count")
                .isGreaterThanOrEqualTo(1);
    }

    private static long countJavaSources(Path dir) throws IOException {
        assertThat(dir).exists().isDirectory();
        try (var stream = Files.walk(dir)) {
            return stream.filter(path -> path.toString().endsWith(".java")).count();
        }
    }
}
