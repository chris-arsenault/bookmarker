package io.ahara.linkdrop.config

import io.ahara.linkdrop.BuildConfig

object LinkdropConfig {
    const val productName = "Linkdrop"
    val apiBaseUrl: String = BuildConfig.LINKDROP_API_BASE_URL.trimEnd('/')
    val cognitoIssuer: String = BuildConfig.COGNITO_ISSUER
    val cognitoDomain: String = BuildConfig.COGNITO_DOMAIN
    val cognitoClientId: String = BuildConfig.COGNITO_CLIENT_ID
    val cognitoRedirectUri: String = BuildConfig.COGNITO_REDIRECT_URI
}
