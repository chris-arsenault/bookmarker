plugins {
    id("com.android.application")
}

val linkdropVersionCode = 1
val linkdropVersionName = "0.1.0"

android {
    namespace = "io.ahara.linkdrop"
    compileSdk = 36

    buildFeatures {
        buildConfig = true
    }

    defaultConfig {
        applicationId = "io.ahara.linkdrop"
        minSdk = 26
        targetSdk = 36
        versionCode = linkdropVersionCode
        versionName = linkdropVersionName

        buildConfigField(
            "String",
            "LINKDROP_API_BASE_URL",
            "\"${providers.gradleProperty("LINKDROP_API_BASE_URL").getOrElse("https://api.linkdrop.ahara.io")}\"",
        )
        buildConfigField(
            "String",
            "COGNITO_ISSUER",
            "\"${providers.gradleProperty("COGNITO_ISSUER").getOrElse("https://cognito-idp.us-east-1.amazonaws.com/us-east-1_XYYtBMb93")}\"",
        )
        buildConfigField(
            "String",
            "COGNITO_DOMAIN",
            "\"${providers.gradleProperty("COGNITO_DOMAIN").getOrElse("auth.services.ahara.io")}\"",
        )
        buildConfigField(
            "String",
            "COGNITO_CLIENT_ID",
            "\"${providers.gradleProperty("COGNITO_CLIENT_ID").getOrElse("241cee9djh44kl9rhc34kbk0ec")}\"",
        )
        buildConfigField(
            "String",
            "COGNITO_REDIRECT_URI",
            "\"${providers.gradleProperty("COGNITO_REDIRECT_URI").getOrElse("io.ahara.linkdrop://auth")}\"",
        )
    }
}

androidComponents {
    onVariants { variant ->
        variant.outputs.forEach { output ->
            val buildMode = if (variant.name == "release") "release-unsigned" else variant.name
            output.outputFileName.set(
                "linkdrop-$buildMode-v$linkdropVersionName-$linkdropVersionCode.apk",
            )
        }
    }
}
