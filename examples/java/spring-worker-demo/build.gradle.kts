plugins {
    java
    application
    id("org.springframework.boot") version "3.5.8"
    id("io.spring.dependency-management") version "1.1.7"
}

group = "com.yhyzgn.tikee.examples"
version = "0.1.0-SNAPSHOT"

dependencies {
    val lombokVersion = "1.18.46"
    compileOnly("org.projectlombok:lombok:$lombokVersion")
    annotationProcessor("org.projectlombok:lombok:$lombokVersion")
    testCompileOnly("org.projectlombok:lombok:$lombokVersion")
    testAnnotationProcessor("org.projectlombok:lombok:$lombokVersion")
    implementation("org.springframework.boot:spring-boot-starter-web")
    implementation("com.yhyzgn.tikee:tikee-spring-boot-starter:0.1.0-SNAPSHOT")
    testImplementation("org.springframework.boot:spring-boot-starter-test")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

application {
    mainClass.set("com.yhyzgn.tikee.examples.worker.SpringWorkerDemoApplication")
}

tasks.withType<Test>().configureEach {
    useJUnitPlatform()
}

tasks.withType<JavaCompile>().configureEach {
    options.release.set(17)
    options.encoding = "UTF-8"
}
