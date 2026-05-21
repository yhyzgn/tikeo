pluginManagement {
    repositories {
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        mavenCentral()
    }
}

rootProject.name = "scheduler-java-sdk"
include(
    "scheduler-java-core",
    "scheduler-spring-boot-autoconfigure",
    "scheduler-spring-boot-starter",
)
