package com.yhyzgn.tikee.examples.worker;

import static org.assertj.core.api.Assertions.assertThat;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;

/** Verifies the demo documents and uses the intended Spring Boot starter compatibility matrix. */
class SpringBootStarterCompatibilityMatrixTest {
    private static final Path DEMO_ROOT = Path.of("").toAbsolutePath();
    private static final Path REPO_ROOT = DEMO_ROOT.resolve("../../..").normalize();
    private static final Path JAVA_SDK_ROOT = REPO_ROOT.resolve("sdks/java");

    @Test
    void demoUsesBoot3CompatibilityStarter() throws IOException {
        String build = Files.readString(DEMO_ROOT.resolve("build.gradle.kts"));
        String readme = Files.readString(DEMO_ROOT.resolve("README.md"));

        assertThat(build)
                .contains("id(\"org.springframework.boot\") version \"3.5.8\"")
                .contains("tikee-spring-boot3-starter")
                .doesNotContain("tikee-spring-boot-starter:0.1.0-SNAPSHOT")
                .doesNotContain("tikee-spring-boot2-starter");
        assertThat(readme)
                .contains("tikee-spring-boot3-starter")
                .contains("Spring Boot 3.x")
                .contains("Spring Boot 2.x")
                .contains("Spring Boot 4.x");
    }

    @Test
    void sdkPublishesSeparateBoot2Boot3AndBoot4StarterModulesWithRealSources() throws IOException {
        String settings = Files.readString(JAVA_SDK_ROOT.resolve("settings.gradle.kts"));
        String sdkReadme = Files.readString(JAVA_SDK_ROOT.resolve("README.md"));

        assertThat(settings)
                .contains("tikee-spring-boot-starter")
                .contains("tikee-spring-boot2-starter")
                .contains("tikee-spring-boot3-starter")
                .contains("tikee-spring5")
                .contains("tikee-spring6");
        assertThat(sdkReadme)
                .contains("primary Spring Boot 4.x starter")
                .contains("Spring Boot 2.x projects")
                .contains("Spring Boot 3.x projects");

        for (String module : List.of(
                "tikee-spring-boot-starter",
                "tikee-spring-boot2-starter",
                "tikee-spring-boot3-starter")) {
            assertRealStarterModule(module);
        }
        for (String module : List.of("tikee-spring", "tikee-spring5", "tikee-spring6")) {
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
