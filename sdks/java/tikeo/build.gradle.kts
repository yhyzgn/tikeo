import com.google.protobuf.gradle.id

plugins {
    `java-library`
    id("com.google.protobuf") version "0.10.0"
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

dependencies {
    api(platform("io.grpc:grpc-bom:${providers.gradleProperty("grpcVersion").get()}"))
    api("io.grpc:grpc-api")
    api("io.grpc:grpc-stub")
    api("io.grpc:grpc-protobuf")
    api("io.grpc:grpc-netty-shaded")
    api("com.google.protobuf:protobuf-java:${providers.gradleProperty("protobufVersion").get()}")
    api("com.fasterxml.jackson.core:jackson-databind:${providers.gradleProperty("jacksonVersion").get()}")
    api("org.slf4j:slf4j-api:${providers.gradleProperty("slf4jVersion").get()}")
    compileOnly("ch.qos.logback:logback-classic:1.2.13")
    compileOnly("javax.annotation:javax.annotation-api:${providers.gradleProperty("javaxAnnotationVersion").get()}")
    compileOnly("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    annotationProcessor("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    testCompileOnly("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    testAnnotationProcessor("org.projectlombok:lombok:${providers.gradleProperty("lombokVersion").get()}")
    testImplementation("org.junit.jupiter:junit-jupiter:${providers.gradleProperty("junitJupiterVersion").get()}")
    testImplementation("ch.qos.logback:logback-classic:1.5.21")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
    testImplementation("io.grpc:grpc-inprocess")
}

protobuf {
    protoc {
        artifact = "com.google.protobuf:protoc:${providers.gradleProperty("protobufVersion").get()}:$protocPlatform@exe"
    }
    plugins {
        id("grpc") {
            artifact = "io.grpc:protoc-gen-grpc-java:${providers.gradleProperty("grpcVersion").get()}:$protocPlatform@exe"
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
