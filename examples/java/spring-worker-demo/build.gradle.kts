plugins {
    java
    application
    id("org.springframework.boot") version "4.0.6"
    id("io.spring.dependency-management") version "1.1.7"
}

group = "cn.recycloud.scheduler.examples"
version = "0.1.0-SNAPSHOT"

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(21))
    }
}


dependencies {
    implementation("org.springframework.boot:spring-boot-starter")
    implementation("cn.recycloud.scheduler:scheduler-spring-boot-starter:0.1.0-SNAPSHOT")
    testImplementation("org.springframework.boot:spring-boot-starter-test")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

application {
    mainClass.set("cn.recycloud.scheduler.examples.worker.SpringWorkerDemoApplication")
}

tasks.withType<Test>().configureEach {
    useJUnitPlatform()
}

tasks.withType<JavaCompile>().configureEach {
    options.release.set(21)
    options.encoding = "UTF-8"
}
