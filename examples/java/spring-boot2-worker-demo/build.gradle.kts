plugins {
    java
    application
}

group = "net.tikeo.examples"
version = "0.1.0-SNAPSHOT"

val springBootVersion = "2.7.18"

configurations.configureEach {
    resolutionStrategy.eachDependency {
        if (requested.group == "org.slf4j" && requested.name == "slf4j-api") {
            useVersion("1.7.36")
            because("Spring Boot 2.7 uses Logback 1.2 / SLF4J 1.7; SLF4J 2 makes logging fall back to NOP")
        }
    }
}

dependencies {
    implementation(platform("org.springframework.boot:spring-boot-dependencies:$springBootVersion"))
    annotationProcessor(platform("org.springframework.boot:spring-boot-dependencies:$springBootVersion"))
    testImplementation(platform("org.springframework.boot:spring-boot-dependencies:$springBootVersion"))

    val lombokVersion = "1.18.46"
    compileOnly("org.projectlombok:lombok:$lombokVersion")
    annotationProcessor("org.projectlombok:lombok:$lombokVersion")
    testCompileOnly("org.projectlombok:lombok:$lombokVersion")
    testAnnotationProcessor("org.projectlombok:lombok:$lombokVersion")
    implementation("org.springframework.boot:spring-boot-starter-web")
    implementation("net.tikeo:tikeo-spring-boot2-starter:0.1.0-SNAPSHOT")
    testImplementation("org.springframework.boot:spring-boot-starter-test")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

application {
    mainClass.set("net.tikeo.examples.worker.SpringWorkerDemoApplication")
}

tasks.withType<Test>().configureEach {
    useJUnitPlatform()
}

tasks.withType<JavaCompile>().configureEach {
    options.release.set(17)
    options.encoding = "UTF-8"
}


// Spring Boot 2.7's Gradle plugin is not compatible with this repo's Gradle 9
// wrapper, so keep the plain application plugin and provide an operator-facing
// bootRun task that behaves like the Boot 3/4 demos instead of aliasing to run.
tasks.register<JavaExec>("bootRun") {
    group = "application"
    description = "Runs this Spring Boot 2 demo with the same command shape as the Boot 3/4 demos."
    classpath = sourceSets.main.get().runtimeClasspath
    mainClass.set("net.tikeo.examples.worker.SpringWorkerDemoApplication")
}
