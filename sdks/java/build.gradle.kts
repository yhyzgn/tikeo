import com.google.protobuf.gradle.id

plugins {
    `java-library`
    id("com.google.protobuf") version "0.9.5" apply false
}

val springBootVersion = "4.0.6"
val grpcVersion = "1.81.0"
val protobufVersion = "4.34.1"
val lombokVersion = "1.18.46"

allprojects {
    group = "cn.recycloud.scheduler"
    version = "0.1.0-SNAPSHOT"
}

subprojects {
    apply(plugin = "java-library")

    java {
        toolchain {
            languageVersion.set(JavaLanguageVersion.of(21))
        }
        withSourcesJar()
    }

    tasks.withType<JavaCompile>().configureEach {
        options.release.set(21)
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

project(":scheduler-java") {
    apply(plugin = "com.google.protobuf")

    dependencies {
        "api"(platform("io.grpc:grpc-bom:$grpcVersion"))
        "api"("io.grpc:grpc-api")
        "api"("io.grpc:grpc-stub")
        "api"("io.grpc:grpc-protobuf")
        "api"("io.grpc:grpc-netty-shaded")
        "api"("com.google.protobuf:protobuf-java:$protobufVersion")
        "compileOnly"("javax.annotation:javax.annotation-api:1.3.2")
        "testImplementation"("io.grpc:grpc-inprocess")
    }

    configure<com.google.protobuf.gradle.ProtobufExtension> {
        protoc {
            artifact = "com.google.protobuf:protoc:$protobufVersion"
        }
        plugins {
            id("grpc") {
                artifact = "io.grpc:protoc-gen-grpc-java:$grpcVersion"
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

project(":scheduler-spring") {
    dependencies {
        "api"(project(":scheduler-java"))
        "api"("org.springframework:spring-context:7.0.2")
        "testImplementation"("org.assertj:assertj-core:3.27.7")
    }
}

project(":scheduler-spring-boot-starter") {
    dependencies {
        "api"(project(":scheduler-spring"))
        "api"(platform("org.springframework.boot:spring-boot-dependencies:$springBootVersion"))
        "api"("org.springframework.boot:spring-boot-starter")
        "api"("org.springframework.boot:spring-boot-autoconfigure")
        "annotationProcessor"(platform("org.springframework.boot:spring-boot-dependencies:$springBootVersion"))
        "annotationProcessor"("org.springframework.boot:spring-boot-configuration-processor")
        "testImplementation"(platform("org.springframework.boot:spring-boot-dependencies:$springBootVersion"))
        "testImplementation"("org.springframework.boot:spring-boot-test")
        "testImplementation"("org.springframework:spring-test")
        "testImplementation"("org.assertj:assertj-core")
    }
}
