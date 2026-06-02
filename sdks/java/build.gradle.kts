import com.google.protobuf.gradle.id

plugins {
    `java-library`
    id("com.google.protobuf") version "0.10.0" apply false
}

val springBoot4Version = "4.0.6"
val springBoot3Version = "3.5.8"
val springBoot2Version = "2.7.18"
val grpcVersion = "1.81.0"
val protobufVersion = "4.34.1"
val lombokVersion = "1.18.46"
val osName = System.getProperty("os.name").lowercase()
val osArch = System.getProperty("os.arch").lowercase()
val protocPlatform = when {
    osName.contains("linux") && (osArch == "amd64" || osArch == "x86_64") -> "linux-x86_64"
    osName.contains("linux") && (osArch == "aarch64" || osArch == "arm64") -> "linux-aarch_64"
    osName.contains("mac") && (osArch == "aarch64" || osArch == "arm64") -> "osx-aarch_64"
    osName.contains("mac") -> "osx-x86_64"
    osName.contains("windows") && (osArch == "amd64" || osArch == "x86_64") -> "windows-x86_64"
    else -> error("Unsupported protoc platform: $osName/$osArch")
}

allprojects {
    group = "com.yhyzgn.tikee"
    version = "0.1.0-SNAPSHOT"
}

subprojects {
    apply(plugin = "java-library")

    java {
        withSourcesJar()
    }

    tasks.withType<JavaCompile>().configureEach {
        options.release.set(17)
        options.encoding = "UTF-8"
    }

    dependencies {
        "compileOnly"("org.projectlombok:lombok:$lombokVersion")
        "annotationProcessor"("org.projectlombok:lombok:$lombokVersion")
        "testCompileOnly"("org.projectlombok:lombok:$lombokVersion")
        "testAnnotationProcessor"("org.projectlombok:lombok:$lombokVersion")
        "testImplementation"("org.junit.jupiter:junit-jupiter:6.0.1")
        "testRuntimeOnly"("org.junit.platform:junit-platform-launcher")
    }

    tasks.withType<Test>().configureEach {
        useJUnitPlatform()
    }
}

project(":tikee") {
    apply(plugin = "com.google.protobuf")

    dependencies {
        "api"(platform("io.grpc:grpc-bom:$grpcVersion"))
        "api"("io.grpc:grpc-api")
        "api"("io.grpc:grpc-stub")
        "api"("io.grpc:grpc-protobuf")
        "api"("io.grpc:grpc-netty-shaded")
        "api"("com.google.protobuf:protobuf-java:$protobufVersion")
        "api"("com.fasterxml.jackson.core:jackson-databind:2.20.1")
        "api"("org.slf4j:slf4j-api:2.0.17")
        "compileOnly"("javax.annotation:javax.annotation-api:1.3.2")
        "testImplementation"("io.grpc:grpc-inprocess")
    }

    configure<com.google.protobuf.gradle.ProtobufExtension> {
        protoc {
            artifact = "com.google.protobuf:protoc:$protobufVersion:$protocPlatform@exe"
        }
        plugins {
            id("grpc") {
                artifact = "io.grpc:protoc-gen-grpc-java:$grpcVersion:$protocPlatform@exe"
            }
        }
        generateProtoTasks {
            all().configureEach {
                plugins {
                    id("grpc")
                }
            }
        }
    }
}

fun Project.configureSpringModule(springContextVersion: String) {
    dependencies {
        "api"(project(":tikee"))
        "api"("org.springframework:spring-context:$springContextVersion")
        "testImplementation"("org.assertj:assertj-core:3.27.7")
    }
}

fun Project.configureBootStarter(springModule: String, bootVersion: String) {
    dependencies {
        "api"(project(springModule))
        "api"(platform("org.springframework.boot:spring-boot-dependencies:$bootVersion"))
        "api"("org.springframework.boot:spring-boot-starter")
        "api"("org.springframework.boot:spring-boot-autoconfigure")
        "annotationProcessor"(platform("org.springframework.boot:spring-boot-dependencies:$bootVersion"))
        "annotationProcessor"("org.springframework.boot:spring-boot-configuration-processor")
        "testImplementation"(platform("org.springframework.boot:spring-boot-dependencies:$bootVersion"))
        "testImplementation"("org.springframework.boot:spring-boot-test")
        "testImplementation"("org.springframework:spring-test")
        "testImplementation"("org.assertj:assertj-core")
    }
}

project(":tikee-spring") {
    configureSpringModule("7.0.2")
}

project(":tikee-spring5") {
    configureSpringModule("5.3.39")
}

project(":tikee-spring6") {
    configureSpringModule("6.2.14")
}

project(":tikee-spring-boot-starter") {
    configureBootStarter(":tikee-spring", springBoot4Version)
}

project(":tikee-spring-boot2-starter") {
    configureBootStarter(":tikee-spring5", springBoot2Version)
}

project(":tikee-spring-boot3-starter") {
    configureBootStarter(":tikee-spring6", springBoot3Version)
}
