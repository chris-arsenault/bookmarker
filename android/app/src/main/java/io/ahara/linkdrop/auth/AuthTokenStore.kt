package io.ahara.linkdrop.auth

import android.content.Context

data class AuthTokens(
    val accessToken: String,
    val refreshToken: String?,
    val expiresAtEpochSeconds: Long,
) {
    fun isFresh(nowEpochSeconds: Long = System.currentTimeMillis() / 1000): Boolean =
        accessToken.isNotBlank() && expiresAtEpochSeconds > nowEpochSeconds + TOKEN_EXPIRY_SKEW_SECONDS

    private companion object {
        const val TOKEN_EXPIRY_SKEW_SECONDS = 60L
    }
}

class AuthTokenStore(context: Context) {
    private val preferences =
        context.getSharedPreferences("linkdrop_auth_tokens", Context.MODE_PRIVATE)

    fun save(tokens: AuthTokens) {
        preferences
            .edit()
            .putString(KEY_ACCESS_TOKEN, tokens.accessToken)
            .putString(KEY_REFRESH_TOKEN, tokens.refreshToken)
            .putLong(KEY_EXPIRES_AT, tokens.expiresAtEpochSeconds)
            .apply()
    }

    fun read(): AuthTokens? {
        val accessToken = preferences.getString(KEY_ACCESS_TOKEN, null) ?: return null
        return AuthTokens(
            accessToken = accessToken,
            refreshToken = preferences.getString(KEY_REFRESH_TOKEN, null),
            expiresAtEpochSeconds = preferences.getLong(KEY_EXPIRES_AT, 0),
        )
    }

    fun clear() {
        preferences.edit().clear().apply()
    }

    private companion object {
        const val KEY_ACCESS_TOKEN = "access_token"
        const val KEY_REFRESH_TOKEN = "refresh_token"
        const val KEY_EXPIRES_AT = "expires_at_epoch_seconds"
    }
}
