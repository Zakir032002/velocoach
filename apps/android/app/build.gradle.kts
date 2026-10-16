plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "dev.velocoach.rig"
    compileSdk = 35

    defaultConfig {
        applicationId = "dev.velocoach.rig"
        minSdk = 31 // BLUETOOTH_ADVERTISE / BLUETOOTH_CONNECT runtime permissions (Android 12)
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"
        ndk { abiFilters += listOf("arm64-v8a") } // libvc_ffi.so is built for arm64 only
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // A demo build: signed with the debug key so it installs without a keystore.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }

    // UniFFI's generated Kotlin and the Rust library come from scripts/build-android.sh.
    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")
}

dependencies {
    // UniFFI's Kotlin bindings load libvc_ffi.so through JNA.
    implementation("net.java.dev.jna:jna:5.14.0@aar")
}
