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

rootProject.name = "tikee-sdk"
include(
    "tikee",
    "tikee-spring",
    "tikee-spring5",
    "tikee-spring6",
    "tikee-spring-boot-starter",
    "tikee-spring-boot2-starter",
    "tikee-spring-boot3-starter",
)
