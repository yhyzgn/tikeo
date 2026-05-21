plugins {
    `java-library`
}

val springBootVersion = "4.0.6"
val grpcVersion = "1.81.0"
val protobufVersion = "4.34.1"

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
        "testImplementation"("org.junit.jupiter:junit-jupiter:6.0.1")
        "testRuntimeOnly"("org.junit.platform:junit-platform-launcher")
    }

    tasks.withType<Test>().configureEach {
        useJUnitPlatform()
    }
}

project(":scheduler-java-core") {
    dependencies {
        "api"(platform("io.grpc:grpc-bom:$grpcVersion"))
        "api"("io.grpc:grpc-api")
        "api"("com.google.protobuf:protobuf-java:$protobufVersion")
    }
}

project(":scheduler-spring-boot-autoconfigure") {
    dependencies {
        "api"(project(":scheduler-java-core"))
        "api"(platform("org.springframework.boot:spring-boot-dependencies:$springBootVersion"))
        "api"("org.springframework.boot:spring-boot-autoconfigure")
        "api"("org.springframework:spring-context")
        "annotationProcessor"(platform("org.springframework.boot:spring-boot-dependencies:$springBootVersion"))
        "annotationProcessor"("org.springframework.boot:spring-boot-configuration-processor")
    }
}

project(":scheduler-spring-boot-starter") {
    dependencies {
        "api"(project(":scheduler-spring-boot-autoconfigure"))
        "api"(platform("org.springframework.boot:spring-boot-dependencies:$springBootVersion"))
        "api"("org.springframework.boot:spring-boot-starter")
    }
}
