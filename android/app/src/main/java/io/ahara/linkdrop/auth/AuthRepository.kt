package io.ahara.linkdrop.auth

interface AuthRepository {
    fun freshBearerToken(): String?
    fun hasFreshToken(): Boolean
}

class StoredTokenAuthRepository(
    private val tokenStore: AuthTokenStore,
) : AuthRepository {
    override fun freshBearerToken(): String? =
        tokenStore.read()?.takeIf(AuthTokens::isFresh)?.accessToken

    override fun hasFreshToken(): Boolean =
        tokenStore.read()?.isFresh() == true
}
