allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.buildDir = "../build"
subprojects {
    project.buildDir = "${rootProject.buildDir}/${project.name}"
}
subprojects {
    project.evaluationDependsOn("${rootProject.project.name}:app")
}

tasks.register("clean", Delete) {
    delete rootProject.buildDir
}