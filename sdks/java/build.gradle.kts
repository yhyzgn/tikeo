import com.vanniktech.maven.publish.JavaLibrary
import com.vanniktech.maven.publish.JavadocJar
import com.vanniktech.maven.publish.SourcesJar

plugins {
    base
    id("com.vanniktech.maven.publish") version "0.36.0" apply false
}

allprojects {
    group = "net.tikeo"
    version = providers.gradleProperty("tikeoVersion").get()
}

subprojects {
    pluginManager.withPlugin("java-library") {
        apply(plugin = "com.vanniktech.maven.publish")

        extensions.configure<com.vanniktech.maven.publish.MavenPublishBaseExtension>("mavenPublishing") {
            configure(
                JavaLibrary(
                    javadocJar = JavadocJar.Empty(),
                    sourcesJar = SourcesJar.Sources(),
                ),
            )
            publishToMavenCentral(automaticRelease = true)
            signAllPublications()

            pom {
                name.set("Tikeo ${project.name}")
                description.set("Tikeo Java SDK module ${project.name} for workflow scheduling workers and management APIs.")
                inceptionYear.set("2026")
                url.set("https://github.com/yhyzgn/tikeo")
                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/license/mit")
                        distribution.set("repo")
                    }
                }
                developers {
                    developer {
                        id.set("yhyzgn")
                        name.set("yhyzgn")
                        url.set("https://github.com/yhyzgn")
                    }
                }
                scm {
                    url.set("https://github.com/yhyzgn/tikeo")
                    connection.set("scm:git:git://github.com/yhyzgn/tikeo.git")
                    developerConnection.set("scm:git:ssh://git@github.com/yhyzgn/tikeo.git")
                }
            }
        }
    }
}
