package net.tikeo.examples.worker;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.assertj.core.api.Assertions;
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

        Assertions.assertThat(build)
                .contains("3.5.8")
                .contains("net.tikeo:tikeo-spring-boot3-starter:0.2.0");
        Assertions.assertThat(readme)
                .contains("Spring Boot 3.x")
                .contains("tikeo-spring-boot3-starter");
    }

    @Test
    void sdkPublishesSeparateBoot2Boot3AndBoot4StarterModulesWithRealSources() throws IOException {
        String settings = Files.readString(JAVA_SDK_ROOT.resolve("settings.gradle.kts"));
        String sdkReadme = Files.readString(JAVA_SDK_ROOT.resolve("README.md"));

        Assertions.assertThat(settings)
                .contains("tikeo-spring-boot-starter")
                .contains("tikeo-spring-boot2-starter")
                .contains("tikeo-spring-boot3-starter")
                .contains("tikeo-spring5")
                .contains("tikeo-spring6");
        Assertions.assertThat(sdkReadme)
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
        Assertions.assertThat(countJavaSources(moduleRoot.resolve("src/main/java")))
                .as(module + " main Java source count")
                .isGreaterThanOrEqualTo(4);
        Assertions.assertThat(countJavaSources(moduleRoot.resolve("src/test/java")))
                .as(module + " test Java source count")
                .isGreaterThanOrEqualTo(1);
        Assertions.assertThat(moduleRoot.resolve("src/main/resources/META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports"))
                .as(module + " Boot 2.7+/3/4 metadata")
                .exists();
        if (module.endsWith("boot2-starter")) {
            Assertions.assertThat(moduleRoot.resolve("src/main/resources/META-INF/spring.factories"))
                    .as(module + " Boot 2 metadata")
                    .exists();
        }
    }

    private static void assertRealSpringAdapterModule(String module) throws IOException {
        Path moduleRoot = JAVA_SDK_ROOT.resolve(module);
        Assertions.assertThat(countJavaSources(moduleRoot.resolve("src/main/java")))
                .as(module + " main Java source count")
                .isGreaterThanOrEqualTo(4);
        Assertions.assertThat(countJavaSources(moduleRoot.resolve("src/test/java")))
                .as(module + " test Java source count")
                .isGreaterThanOrEqualTo(1);
    }

    private static long countJavaSources(Path dir) throws IOException {
        Assertions.assertThat(dir).exists().isDirectory();
        try (var stream = Files.walk(dir)) {
            return stream.filter(path -> path.toString().endsWith(".java")).count();
        }
    }
}
