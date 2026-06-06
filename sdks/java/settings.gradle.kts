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

rootProject.name = "tikeo-sdk"
include(
    "tikeo",
    "tikeo-spring",
    "tikeo-spring5",
    "tikeo-spring6",
    "tikeo-spring-boot-starter",
    "tikeo-spring-boot2-starter",
    "tikeo-spring-boot3-starter",
)
