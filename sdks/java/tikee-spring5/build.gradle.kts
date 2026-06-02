plugins {
    `java-library`
    `maven-publish`
}

java {
    withSourcesJar()
}

tasks.withType<JavaCompile>().configureEach {
    options.release.set(providers.gradleProperty("javaRelease").get().toInt())
    options.encoding = "UTF-8"
}

tasks.withType<Test>().configureEach {
    useJUnitPlatform()
}

dependencies {
    api(project(":tikee"))
    api("org.springframework:spring-context:${providers.gradleProperty("springFramework5Version").get()}")
    compileOnly("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    annotationProcessor("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    testCompileOnly("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    testAnnotationProcessor("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    testImplementation("org.junit.jupiter:junit-jupiter:${providers.gradleProperty("junitJupiterVersion").get()}")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
    testImplementation("org.assertj:assertj-core:${providers.gradleProperty("assertjVersion").get()}")
}

publishing {
    publications {
        create<MavenPublication>("mavenJava") {
            from(components["java"])
        }
    }
}
