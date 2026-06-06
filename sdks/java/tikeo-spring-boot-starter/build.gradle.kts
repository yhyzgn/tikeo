plugins {
    `java-library`
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
    api(project(":tikeo-spring"))
    api(platform("org.springframework.boot:spring-boot-dependencies:${providers.gradleProperty("springBoot4Version").get()}"))
    api("org.springframework.boot:spring-boot-starter")
    api("org.springframework.boot:spring-boot-autoconfigure")
    annotationProcessor(platform("org.springframework.boot:spring-boot-dependencies:${providers.gradleProperty("springBoot4Version").get()}"))
    annotationProcessor("org.springframework.boot:spring-boot-configuration-processor")
    compileOnly("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    annotationProcessor("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    testCompileOnly("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    testAnnotationProcessor("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    testImplementation("org.junit.jupiter:junit-jupiter:${providers.gradleProperty("junitJupiterVersion").get()}")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
    testImplementation(platform("org.springframework.boot:spring-boot-dependencies:${providers.gradleProperty("springBoot4Version").get()}"))
    testImplementation("org.springframework.boot:spring-boot-test")
    testImplementation("org.springframework:spring-test")
    testImplementation("org.assertj:assertj-core")
}
