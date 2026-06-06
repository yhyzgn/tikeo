plugins {
    base
}

allprojects {
    group = "net.tikeo"
    version = providers.gradleProperty("tikeoVersion").get()
}
