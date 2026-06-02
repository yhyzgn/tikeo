plugins {
    base
}

allprojects {
    group = "com.yhyzgn.tikee"
    version = providers.gradleProperty("tikeeVersion").get()
}
